use godot::engine::RenderingServer;
use godot::prelude::*;

#[inline]
pub fn rs() -> Gd<RenderingServer> {
    RenderingServer::singleton()
}
