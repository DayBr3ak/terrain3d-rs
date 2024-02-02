use godot::prelude::*;

mod player;
mod player2;

mod terrain_3d;

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
