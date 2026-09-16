mod inmueble;
mod note;
mod solicitud;
mod user;

pub use inmueble::{InmuebleRepository, NuevoInmueble};
pub use note::NoteRepository;
pub use solicitud::{NuevaSolicitud, SolicitudRepository};
pub use user::UserRepository;
