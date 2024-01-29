use godot::prelude::*;

mod player;
mod player2;

mod terrain_3d;
use terrain_3d::terrain_3d_core;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
