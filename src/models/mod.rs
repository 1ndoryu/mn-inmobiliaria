mod inmueble;
mod note;
mod user;

pub use inmueble::{
    AddFotoRequest, CopyInmueble, CreateInmuebleRequest, CreateUserRequest, FiltrosPublicos, Foto,
    FotoPublica, Inmueble, InmuebleRow, PaginatedInmuebles, PublicacionRequest,
    UpdateInmuebleRequest, ESTADOS, EXTENSIONES_FOTO, MAX_FOTO_BYTES, OPERACIONES, ORIGENES_FOTO,
    TIPOS,
};

pub use note::{CreateNoteRequest, Note, PaginatedNotes, PaginationParams, UpdateNoteRequest};
pub use user::{AuthResponse, LoginRequest, RegisterRequest, User, UserResponse};
