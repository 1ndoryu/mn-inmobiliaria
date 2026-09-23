"""Worker de mejora: sube la foto original + prompt a Gemini web y guarda la mejorada.

Dos modos (lo invoca server/mejora.mjs, no a mano):

- Una foto:
    python server/gemini_worker.py --input <foto> --output <mejorada> --prompt "..."
  Contrato stdout: imprime "OK <ruta>" o "NO_IMAGE ..." / "ERROR ...".

- Daemon (una sola sesión para todas las fotos, sin login por foto):
    python server/gemini_worker.py --daemon
  Lee peticiones JSON por stdin (una por línea):
    {"id": "abc", "input": "<foto>", "output": "<mejorada>", "prompt": "..."}
  Responde JSON por stdout (una línea por petición):
    {"id": "abc", "ok": true}
    {"id": "abc", "ok": false, "error": "..."}
  Sonda de credenciales (F21, sin gastar foto):
    {"id": "p1", "probar": true}  ->  {"id": "p1", "ok": true} / {"id": "p1", "ok": false, ...}
  Tras iniciar sesión imprime: {"listo": true, "sesion": "nueva"}.
  Ante un fallo de sesión a mitad de un trabajo, re-inicia sesión una vez
  y reintenta ese trabajo antes de rendirse.

Las cookies salen de GEMINI_PSID / GEMINI_PSIDTS (nunca en argv ni en el repo).
Los logs humanos van a stderr; por stdout solo protocolo (JSON en daemon,
"OK ..."/"ERROR ..." en una foto).
"""

import argparse
import asyncio
import json
import os
import sys
from pathlib import Path


def _log(msg: str) -> None:
    print(msg, file=sys.stderr, flush=True)


async def _bajar_mejorada(img, salida: str, client) -> None:
    from gemini_webapi.constants import Headers

    # Diagnóstico de resolución (2026-09-23): el botón Descargar de la web trae
    # más píxeles que el render inline. Se prueban 3 variantes y se queda la
    # mayor por píxeles: tal cual (RPC), =s0 (tamaño original, como Descargar)
    # y =s2048-rj. Solo se loguean dimensiones y KB, nunca URLs ni cookies.
    try:
        from PIL import Image as _PILImage
        import io as _io

        def _dims(d: bytes):
            try:
                with _PILImage.open(_io.BytesIO(d)) as im:
                    return im.size
            except Exception:
                return None
    except ImportError:
        def _dims(d: bytes):
            return None

    cabeceras = dict(Headers.REFERER.value)
    url_final = img.url
    # 1. URL full-size vía RPC (la misma que usa el botón Descargar de la web).
    try:
        if all([getattr(img, 'client_ref', None), getattr(img, 'cid', ''), getattr(img, 'rid', ''),
                getattr(img, 'rcid', ''), getattr(img, 'image_id', '')]):
            original = await img.client_ref._get_full_size_image(
                cid=img.cid, rid=img.rid, rcid=img.rcid, image_id=img.image_id)
            if original:
                sesion = client.client
                r1 = await sesion.get(f"{original}=d-I?alr=yes", headers=cabeceras)
                r1.raise_for_status()
                sesion = client.client  # re-captura por si el RPC rotó la sesión
                r2 = await sesion.get(r1.text, headers=cabeceras)
                r2.raise_for_status()
                url_final = r2.text
            else:
                _log("full-size RPC devolvió vacío, uso URL directa.")
        else:
            _log("full-size RPC omitido (sin refs de imagen), uso URL directa.")
    except Exception as e:
        _log(f"AVISO full-size RPC falló ({type(e).__name__}), uso URL directa.")
    # 2. Variantes de resolución sobre la URL base (sin sufijo previo).
    import re as _re
    base = _re.sub(r"=s\d+(-rj)?$", "", url_final)
    candidatas = [("talcual", url_final), ("s0", base + "=s0"), ("s2048", base + "=s2048-rj")]
    # 3. Descarga cada variante con la sesión actual (viva) y gana la mayor.
    mejor = None  # (pixeles, nombre, bytes)
    for nombre, url in candidatas:
        try:
            sesion = client.client
            if sesion is None:
                raise RuntimeError("La sesión de Gemini se cerró antes de descargar.")
            respuesta = await sesion.get(url, headers=cabeceras)
            respuesta.raise_for_status()
            datos = respuesta.content
            d = _dims(datos)
            px = d[0] * d[1] if d else 0
            _log(f"descarga: {nombre} {d} {len(datos) // 1024}KB")
            if mejor is None or px > mejor[0]:
                mejor = (px, nombre, datos)
        except Exception as e:
            _log(f"descarga: {nombre} falló ({type(e).__name__}).")
    if mejor is None:
        raise RuntimeError("No se pudo descargar la imagen en ninguna variante.")
    _log(f"descarga: elegida {mejor[1]}.")
    Path(salida).write_bytes(mejor[2])


async def _iniciar_sesion():
    from gemini_webapi import GeminiClient

    psid = os.getenv("GEMINI_PSID", "")
    psidts = os.getenv("GEMINI_PSIDTS", "") or None
    if not psid:
        raise RuntimeError("Falta GEMINI_PSID en el entorno del backend.")
    client = GeminiClient(psid, psidts)
    await client.init(timeout=30, auto_close=False, close_delay=300, auto_refresh=True)
    return client


async def _procesar(client, entrada: str, salida: str, prompt: str) -> None:
    chat = client.start_chat()
    # Workaround verificado 2026-09-14: mandar el archivo en el PRIMER
    # mensaje del chat cierra la sesión (SessionClosed); un texto previo
    # lo evita y el segundo mensaje con files funciona.
    await chat.send_message("Preparo una foto para mejorar.", temporary=True)
    respuesta = await chat.send_message(prompt, files=[entrada], temporary=True)
    if not respuesta.images:
        texto = (respuesta.text or "").strip().replace("\n", " ")[:200]
        raise RuntimeError(f"Gemini no devolvió imagen. {texto}")
    # La mejor resolución disponible: se guarda tal cual, sin recomprimir.
    await _bajar_mejorada(respuesta.images[0], salida, client)


async def _mejorar_una(entrada: str, salida: str, prompt: str) -> None:
    client = await _iniciar_sesion()
    try:
        await _procesar(client, entrada, salida, prompt)
    finally:
        try:
            await client.close()
        except Exception:
            pass
    print(f"OK {salida}")


async def _daemon() -> None:
    """Una sesión para todas las fotos: login una vez, un chat nuevo por foto."""
    client = await _iniciar_sesion()
    trabajos = 0
    print(json.dumps({"listo": True, "sesion": "nueva"}), flush=True)
    _log("daemon: sesión iniciada, esperando peticiones por stdin.")
    bucle = asyncio.get_running_loop()
    while True:
        linea = await bucle.run_in_executor(None, sys.stdin.readline)
        if not linea:
            break  # stdin cerrado: el backend nos ha dado de baja
        try:
            pet = json.loads(linea)
        except (json.JSONDecodeError, KeyError) as e:
            _log(f"daemon: petición inválida, se ignora ({type(e).__name__}).")
            continue
        # F21: sonda de credenciales sin gastar una foto. Login fresco +
        # un mensaje de texto temporal (sin imagen): si las cookies no
        # autentican, el RPC falla aquí mismo en segundos.
        # F26: el texto NO pasa por el gate de imágenes de la librería, así
        # que un OK de texto con sesión degradada es falso positivo. Se
        # devuelve el estado que Google dio al login (AVAILABLE = imágenes OK).
        if pet.get("probar"):
            pid = pet.get("id") or "probar"
            try:
                sonda = await _iniciar_sesion()
                estado = sonda.account_status.name
                try:
                    chat = sonda.start_chat()
                    await chat.send_message("Responde solo con la palabra OK.", temporary=True)
                finally:
                    try:
                        await sonda.close()
                    except Exception:
                        pass
                print(json.dumps({"id": pid, "ok": True, "estado": estado}), flush=True)
            except Exception as e:
                _log(f"daemon: prueba ERROR {type(e).__name__}.")
                print(json.dumps({"id": pid, "ok": False, "error": f"{type(e).__name__}: {str(e)[:300]}"}), flush=True)
            continue
        try:
            pid, entrada, salida, prompt = pet["id"], pet["input"], pet["output"], pet["prompt"]
        except (json.JSONDecodeError, KeyError) as e:
            _log(f"daemon: petición inválida, se ignora ({type(e).__name__}).")
            continue
        try:
            try:
                await _procesar(client, entrada, salida, prompt)
            except Exception as primera:
                # La sesión puede haber caducado entre fotos: un re-login y
                # un solo reintento antes de dar el trabajo por perdido.
                _log(f"daemon: fallo ({type(primera).__name__}), re-inicio sesión y reintento.")
                try:
                    await client.close()
                except Exception:
                    pass
                client = await _iniciar_sesion()
                await _procesar(client, entrada, salida, prompt)
            trabajos += 1
            _log(f"daemon: trabajo {pid} OK (sesión reutilizada x{trabajos}).")
            print(json.dumps({"id": pid, "ok": True}), flush=True)
        except Exception as e:
            _log(f"daemon: trabajo {pid} ERROR {type(e).__name__}.")
            print(json.dumps({"id": pid, "ok": False, "error": f"{type(e).__name__}: {str(e)[:300]}"}), flush=True)
    try:
        await client.close()
    except Exception:
        pass


def main() -> None:
    parser = argparse.ArgumentParser(description="Mejora fotos con Gemini web.")
    parser.add_argument("--daemon", action="store_true", help="Sesión persistente por stdin (JSON por línea).")
    parser.add_argument("--input", help="Foto original (png/jpg).")
    parser.add_argument("--output", help="Destino de la mejorada.")
    parser.add_argument("--prompt", help="Prompt de mejora.")
    args = parser.parse_args()
    if args.daemon:
        asyncio.run(_daemon())
        return
    if not (args.input and args.output and args.prompt):
        parser.error("--input/--output/--prompt son obligatorios sin --daemon.")
    asyncio.run(_mejorar_una(args.input, args.output, args.prompt))


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception as e:  # Error explícito, nunca silencioso.
        print(f"ERROR {type(e).__name__}: {str(e)[:300]}")
        sys.exit(1)
