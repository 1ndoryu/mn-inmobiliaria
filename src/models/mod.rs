mod inmueble;
mod note;
mod user;

pub use inmueble::{
    AddFotoRequest, CreateInmuebleRequest, CreateUserRequest, FiltrosPublicos, Foto, Inmueble,
    InmuebleRow, PaginatedInmuebles, PublicacionRequest, UpdateInmuebleRequest, ESTADOS,
    OPERACIONES, ORIGENES_FOTO, TIPOS,
};

pub use note::{CreateNoteRequest, Note, PaginatedNotes, PaginationParams, UpdateNoteRequest};
pub use user::{AuthResponse, LoginRequest, RegisterRequest, User, UserResponse};
