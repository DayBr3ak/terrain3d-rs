extends MeshInstance3D


# Called when the node enters the scene tree for the first time.
func _ready():
	var x = mesh.surface_get_material(0)
	
	for i in range(1, mesh.get_surface_count()):
		mesh.surface_set_material(i, x)
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(_delta):
	pass
