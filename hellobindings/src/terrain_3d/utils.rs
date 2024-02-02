use godot::engine::RenderingServer;
use godot::prelude::*;

#[inline]
pub fn rs() -> Gd<RenderingServer> {
    RenderingServer::singleton()
}

#[macro_export]
macro_rules! log_error {
    ($self:ident, $fmt:literal $(, $args:expr)* $(,)?) => {
        let s = format!($fmt $(, $args)*);
        crate::godot_print!("[ERR]  {}:: {}", $self::__CLASS__, s);
        crate::godot_error!("[ERR]  {}:: {}", $self::__CLASS__, s)
    };
}

#[macro_export]
macro_rules! log_info {
    ($self:ident, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::godot_print!("[INFO] {}:: {}", $self::__CLASS__, format!($fmt $(, $args)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($self:ident, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::godot_print!("[DBG]  {}:: {}", $self::__CLASS__, format!($fmt $(, $args)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($self:ident, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::godot_print!("[WARN] {}:: {}", $self::__CLASS__, format!($fmt $(, $args)*))
    };
}

