#![cfg(test)]

use super::InmuebleService;

#[test]
fn extension_y_magia() {
    assert!(InmuebleService::extension_valida("foto.JPG").is_ok());
    assert!(InmuebleService::extension_valida("foto.webp").is_ok());
    assert!(InmuebleService::extension_valida("sin-extension").is_err());
    assert!(InmuebleService::extension_valida("doc.pdf").is_err());
    let jpeg = [0xFF, 0xD8, 0xFF, 0x00];
    assert!(InmuebleService::magia_valida(&jpeg, ".jpg").is_ok());
    assert!(InmuebleService::magia_valida(&jpeg, ".png").is_err());
    let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    assert!(InmuebleService::magia_valida(&png, ".png").is_ok());
    assert!(InmuebleService::magia_valida(b"RIFFxxxxWEBP", ".webp").is_ok());
    assert!(InmuebleService::magia_valida(&[], ".jpg").is_err());
}

/* [299A-3] Notas de voz: extensiones + firmas mínimas por formato. */
#[test]
fn extension_y_magia_audio() {
    assert!(InmuebleService::extension_audio_valida("nota.OGG").is_ok());
    assert!(InmuebleService::extension_audio_valida("nota.mp3").is_ok());
    assert!(InmuebleService::extension_audio_valida("nota.m4a").is_ok());
    assert!(InmuebleService::extension_audio_valida("nota.wav").is_err());
    assert!(InmuebleService::extension_audio_valida("nota.jpg").is_err());
    assert!(InmuebleService::magia_audio_valida(b"OggSxyz", ".ogg").is_ok());
    assert!(InmuebleService::magia_audio_valida(b"ID3xyz", ".mp3").is_ok());
    assert!(InmuebleService::magia_audio_valida(&[0xFF, 0xFB, 0x90], ".mp3").is_ok());
    assert!(InmuebleService::magia_audio_valida(b"xxxxftypm4a ", ".m4a").is_ok());
    assert!(InmuebleService::magia_audio_valida(b"OggSxyz", ".mp3").is_err());
    assert!(InmuebleService::magia_audio_valida(&[], ".ogg").is_err());
}
