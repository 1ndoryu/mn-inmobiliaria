mod inmueble;
mod note;
mod solicitud;
mod user;

pub use inmueble::{
    AddFotoRequest, CopyInmueble, CreateInmuebleRequest, CreateUserRequest, FiltrosPublicos, Foto,
    FotoPublica, Inmueble, InmuebleRow, PaginatedInmuebles, PublicacionRequest,
    UpdateInmuebleRequest, ESTADOS, EXTENSIONES_FOTO, MAX_FOTO_BYTES, OPERACIONES, ORIGENES_FOTO,
    TIPOS,
};

pub use note::{CreateNoteRequest, Note, PaginatedNotes, PaginationParams, UpdateNoteRequest};
pub use solicitud::{
    ConteoEstado, CreateSolicitudRequest, FiltrosSolicitudes, FotoSolicitud, FotoSolicitudSubida,
    PaginatedSolicitudes, Solicitud, SolicitudRow, SolicitudesAdminParams, StatsSolicitudes,
    UpdateEstadoSolicitud, ESTADOS_SOLICITUD, MAX_FOTOS_SOLICITUD, ORIGENES_CONTACTO,
};
pub use user::{AuthResponse, LoginRequest, RegisterRequest, User, UserResponse};
