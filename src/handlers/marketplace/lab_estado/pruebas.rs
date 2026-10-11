use super::*;

#[test]
fn sin_historial_es_sin_senal() {
    assert_eq!(clasificar(1_000, 0, 0), EstadoLab::SinSenal);
}

#[test]
fn exito_reciente_es_conectado() {
    assert_eq!(clasificar(1_000, 990, 0), EstadoLab::Conectado);
    assert_eq!(clasificar(1_000, 1_000 - VENTANA_CONECTADO_S, 0), EstadoLab::Conectado);
}

#[test]
fn exito_viejo_es_sin_senal() {
    assert_eq!(clasificar(1_000, 1_000 - VENTANA_CONECTADO_S - 1, 0), EstadoLab::SinSenal);
}

#[test]
fn rechazo_mas_nuevo_que_el_exito_es_rechazado() {
    assert_eq!(clasificar(1_000, 980, 995), EstadoLab::Rechazado);
    assert_eq!(clasificar(1_000, 0, 995), EstadoLab::Rechazado);
}

#[test]
fn el_lab_se_recupera_tras_un_rechazo() {
    assert_eq!(clasificar(1_000, 998, 990), EstadoLab::Conectado);
}

#[test]
fn empate_en_el_mismo_segundo_gana_el_exito() {
    assert_eq!(clasificar(1_000, 995, 995), EstadoLab::Conectado);
}

#[test]
fn rechazo_caducado_deja_de_alarmar() {
    assert_eq!(clasificar(1_000, 0, 1_000 - VENTANA_RECHAZO_S - 1), EstadoLab::SinSenal);
}
