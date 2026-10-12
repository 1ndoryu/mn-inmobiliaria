/* [08AA-7] Singleflight del asistente Marketplace en su dominio: N
 * peticiones concurrentes con la misma clave (doble clic en Regenerar,
 * reintento + original en vuelo) comparten UNA generación de IA en vez de
 * gastar N. El líder computa y difunde; los seguidores esperan el mismo
 * `Arc`. Ventana residual de microsegundos entre `send` y `remove` (el que
 * llegue ahí recomputa): best-effort honesto para doble clic humano, no
 * barrera distribuida; el `UNIQUE` + `DO NOTHING` de la tabla respalda
 * duplicados. Solo vive en memoria del proceso.
 * [08AA-6] Fan-out con `mpsc::unbounded_channel` por seguidor en vez de
 * `broadcast`: `broadcast::Sender::send` toma un `std::Mutex` interno que
 * bloquea workers tokio bajo contención; el `send` unbounded nunca espera
 * (un mensaje por seguidor, acotado por definición) y el `drop` de la lista
 * si el líder cae degrada a fallback igual que antes (fail-open).
 * Se re-exporta desde `marketplace.rs` para no mover sus 4 usos externos. */

use super::{asegurar_contacto, Coste, FALLBACK_BORRADOR};

/// Generación compartible en vuelo: texto + fuente (`ia`, nunca `reserva` —
/// el fallback no entra al vuelo: cada miss reintenta la IA) + coste de la
/// pasada (`Coste::default()` si no hubo IA).
#[derive(Debug, Clone)]
pub struct Generado {
    pub texto: String,
    pub fuente: String,
    pub coste: Coste,
}

type Mapa = std::collections::HashMap<
    String,
    Vec<tokio::sync::mpsc::UnboundedSender<std::sync::Arc<Generado>>>,
>;

#[derive(Debug, Default)]
pub struct Singleflight {
    /* `std` y no `tokio`: el lock nunca se sostiene a través de un `await`, y
     * así `Drop` de `Vuelo` puede liberar la clave de forma síncrona. */
    vuelo: std::sync::Mutex<Mapa>,
}

/* [09AA-36a] Guarda del líder: si el líder cae antes de difundir (el cliente
 * cierra la conexión y Axum suelta el future), `Drop` libera la clave. Sin
 * ella la entrada quedaba para siempre y cada petición posterior con la misma
 * clave se encolaba como seguidora esperando a un líder que ya no existe. */
struct Vuelo<'a> {
    sf: &'a Singleflight,
    clave: &'a str,
    difundido: bool,
}

impl Vuelo<'_> {
    fn difundir(&mut self, gen: &std::sync::Arc<Generado>) {
        let lista = self.sf.mapa().remove(self.clave).unwrap_or_default();
        for tx in lista {
            let _ = tx.send(std::sync::Arc::clone(gen));
        }
        /* Tras difundir la clave puede tener ya otro líder: no tocarla al caer. */
        self.difundido = true;
    }
}

impl Drop for Vuelo<'_> {
    fn drop(&mut self) {
        if !self.difundido {
            self.sf.mapa().remove(self.clave);
        }
    }
}

impl Singleflight {
    fn mapa(&self) -> std::sync::MutexGuard<'_, Mapa> {
        /* Un pánico ajeno no debe dejar la clave bloqueada para siempre. */
        self.vuelo
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Ejecuta `f` si nadie vuela con `clave`; si no, espera el resultado
    /// ajeno. Sin `Send` en `f`: se sondea inline, sin `spawn`.
    pub async fn ejecutar<F, Fut>(&self, clave: &str, f: F) -> std::sync::Arc<Generado>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Generado>,
    {
        let seguidor = {
            let mut mapa = self.mapa();
            if let Some(lista) = mapa.get_mut(clave) {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                lista.push(tx);
                Some(rx)
            } else {
                mapa.insert(clave.to_string(), Vec::new());
                None
            }
        };
        /* Seguidor: el líder difunde; si el líder cayó (su lista se dropeó
         * con los `Sender`), se degrada a fallback en vez de colgar
         * (fail-open). */
        if let Some(mut rx) = seguidor {
            rx.recv().await.unwrap_or_else(|| {
                std::sync::Arc::new(Generado {
                    texto: asegurar_contacto(FALLBACK_BORRADOR),
                    fuente: "reserva".to_string(),
                    coste: Coste::default(),
                })
            })
        } else {
            let mut vuelo = Vuelo {
                sf: self,
                clave,
                difundido: false,
            };
            let gen = std::sync::Arc::new(f().await);
            vuelo.difundir(&gen);
            gen
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /* Singleflight (assert del plan): 10 concurrentes con la misma clave =
     * UNA sola ejecución y el mismo `Arc` para todos. */
    #[tokio::test]
    async fn vuelo_comparte_una_generacion() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let vuelo = Singleflight::default();
        let vuelo = Arc::new(vuelo);
        let contador = Arc::new(AtomicUsize::new(0));
        let mut tareas = Vec::new();
        for _ in 0..10 {
            let v = Arc::clone(&vuelo);
            let c = Arc::clone(&contador);
            tareas.push(tokio::spawn(async move {
                v.ejecutar("clave-x", || async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    Generado {
                        texto: "hola".to_string(),
                        fuente: "ia".to_string(),
                        coste: Coste::default(),
                    }
                })
                .await
            }));
        }
        let mut resultados = Vec::new();
        for t in tareas {
            resultados.push(t.await.expect("tarea"));
        }
        assert_eq!(contador.load(Ordering::SeqCst), 1);
        for r in &resultados {
            assert_eq!(r.texto, "hola");
            assert!(Arc::ptr_eq(&resultados[0], r), "mismo Arc para todos");
        }
    }

    /* [09AA-36a] Líder cancelado (cliente desconectado) a mitad de generación:
     * el seguidor cae a fallback y una petición nueva con la misma clave vuelve
     * a generar. Antes del `Drop` la clave quedaba colgada y esto no terminaba. */
    #[tokio::test]
    async fn lider_cancelado_libera_la_clave() {
        use std::sync::Arc;
        use std::time::Duration;
        let vuelo = Arc::new(Singleflight::default());
        let v = Arc::clone(&vuelo);
        let lider = tokio::spawn(async move {
            v.ejecutar("clave-y", || async {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                Generado {
                    texto: "nunca".to_string(),
                    fuente: "ia".to_string(),
                    coste: Coste::default(),
                }
            })
            .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        let v = Arc::clone(&vuelo);
        let seguidor = tokio::spawn(async move {
            v.ejecutar("clave-y", || async {
                Generado {
                    texto: "no-debe-ejecutarse".to_string(),
                    fuente: "ia".to_string(),
                    coste: Coste::default(),
                }
            })
            .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        lider.abort();
        let _ = lider.await;
        let r = tokio::time::timeout(Duration::from_secs(2), seguidor)
            .await
            .expect("el seguidor no debe colgar")
            .expect("tarea seguidora");
        assert_eq!(r.fuente, "reserva", "seguidor sin líder cae a fallback");
        let nuevo = tokio::time::timeout(
            Duration::from_secs(2),
            vuelo.ejecutar("clave-y", || async {
                Generado {
                    texto: "nuevo".to_string(),
                    fuente: "ia".to_string(),
                    coste: Coste::default(),
                }
            }),
        )
        .await
        .expect("la clave debe estar libre");
        assert_eq!(nuevo.texto, "nuevo");
    }
}
