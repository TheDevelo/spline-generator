require 'mittsu'

# Spline settings
SAMPLES_PER_SEGMENT = 16
SPLINE_LENGTH_INCREMENT = 16.0
GRID_SNAP = 16.0
ANGLE_SNAP_DEG = 5.0
ANGLE_SNAP_RAD = ANGLE_SNAP_DEG * Math::PI / 180.0

# Render settings
SCREEN_WIDTH = 1280
SCREEN_HEIGHT = 720
ASPECT = SCREEN_WIDTH.to_f / SCREEN_HEIGHT.to_f
TEXTURE_SCALE = 256.0
PIXELS_PER_180 = 1000.0
NOCLIP_SPEED = 8.0
RENDER_DISTANCE = 4096.0
CULL_FACTOR = 2.0
CULL_UPDATE_RATE = 30

if (ARGV.length != 1 and ARGV.length != 2)
  puts "Usage: ruby spline.rb MAP.vmf [SPLINE_SAVE.txt]"
  return
end

if not File.file?(ARGV[0])
  puts "ERROR: specify a valid vmf"
  puts "Usage: ruby spline.rb MAP.vmf [SPLINE_SAVE.txt]"
  return
end

vmf = File.readlines(ARGV[0])
vmf = vmf.map {|l| l.lstrip.rstrip}
vmf = vmf.join("\n").gsub("\n{", " {").split("\n")

class Directory
  attr_accessor :parent

  def initialize(parent)
    @parent = parent
    @entries = {}
  end

  def insert(name, value)
    @entries[name] ||= []
    @entries[name] << value
  end

  def get_all(name)
    result = @entries[name]
    return [] if result.nil?
    return result
  end

  def get_first(name)
    get_all(name)[0]
  end
end

# Parse VMF into useable format
entry_regex = /^"(.*)" "(.*)"$/
directory_regex = /^(.*) {$/
parsed_vmf = Directory.new nil
current_dir = parsed_vmf
vmf.each do |line|
  entry_match = line.match entry_regex
  directory_match = line.match directory_regex

  if line == "}"
    current_dir = current_dir.parent
  elsif not entry_match.nil?
    current_dir.insert(entry_match[1], entry_match[2])
  elsif not directory_match.nil?
    new_dir = Directory.new current_dir
    current_dir.insert(directory_match[1], new_dir)
    current_dir = new_dir
  else
    puts "ERROR: Line malformed - Not an entry, directory, or closing brace"
    puts "ERRORing line: #{line}"
    return
  end
end

# Get the solids we want to render
sides = parsed_vmf.get_first("world").get_all("solid").map {|s| s.get_all("side")}.flatten
sides += parsed_vmf.get_all("entity")
  .select {|e| e.get_first("classname") == "func_detail" }
  .map {|e| e.get_all("solid").map {|s| s.get_all("side")}.flatten }
  .flatten
sides = sides.select do |s|
  material = s.get_first("material").upcase
  material != "TOOLS/TOOLSNODRAW" and
    material != "TOOLS/TOOLSPLAYERCLIP"
end

sides_partition = sides.partition do |s|
  material = s.get_first("material").upcase
  material == "TOOLS/TOOLSSKYBOX" or material == "TOOLS/TOOLSSKYBOX2D"
end
sky_sides = sides_partition[0]
solid_sides = sides_partition[1]

# Load spline save file if specified
save_loaded = false
spline = []
spline_length = 128.0
current_point = 0

if ARGV.length == 2
  if not File.file?(ARGV[1])
    puts "ERROR: specify a valid save"
    puts "Usage: ruby spline.rb MAP.vmf [SPLINE_SAVE.txt]"
    return
  end

  spline_save = File.readlines(ARGV[1])

  float_regex = /^[-+]?(?:[0-9]*\.[0-9]+|[0-9]+)$/
  spline_save.each_with_index do |line, line_num|
    nums = line.split(' ')
    if nums.length != 6
      puts "ERROR: Line #{line_num} malformed - Incorrect # of parameters"
      return
    end
    nums.each do |n|
      unless float_regex.match?(n)
        puts "ERROR: Line #{line_num} malformed - Non-FP parameter"
        return
      end
    end

    nums = nums.map {|n| n.to_f }
    if nums[3] >= 360.0 or nums[3] < 0.0
      puts "ERROR: XY angle on line #{line_num} out of range (0.0 <= x < 360.0)"
      return
    end
    if nums[4] > 90.0 or nums[4] < -90.0
      puts "ERROR: Z angle on line #{line_num} out of range (-90.0 <= x <= 90.0)"
      return
    end
    if nums[5] < 0.0
      puts "ERROR: Control length on line #{line_num} out of range (0.0 <= x)"
      return
    end
    nums[3] = nums[3] * Math::PI / 180.0
    nums[4] = nums[4] * Math::PI / 180.0

    spline << nums
  end

  spline_length = spline[-1][5]
  current_point = spline.length
  save_loaded = true
end

# Initialize rendering
renderer = Mittsu::OpenGLRenderer.new width: SCREEN_WIDTH, height: SCREEN_HEIGHT, title: 'Spline Helper'
scene = Mittsu::Scene.new
camera = Mittsu::PerspectiveCamera.new(90.0, ASPECT, 1.0, RENDER_DISTANCE)

# Construct Mittsu meshes from the VMF solids
# Uses Hammer++ specific fields so that I don't have to calculate plane intersections
# Just open and save a VMF through Hammer++ to generate said fields
def construct_geometry_from_side(side)
  verts = side.get_first("vertices_plus").get_all("v").map {|v| v.split(" ").map {|s| s.to_f }}
  offset = verts.transpose.map {|c| c.sum / c.length.to_f }
  verts = verts.map {|v| v.zip(offset).map {|p| p[0] - p[1]}}
  triangulated_side = []
  (1...(verts.length - 1)).each do |i|
    triangulated_side << verts[0]
    triangulated_side << verts[i+1]
    triangulated_side << verts[i]
  end

  # Manually construct a geometry object as desired
  geometry = Mittsu::Geometry.new
  geometry.vertices = triangulated_side.map {|v| Mittsu::Vector3.new(v[0], v[1], v[2]) }
  (0...(triangulated_side.length / 3)).each do |i|
    geometry.faces << Mittsu::Face3.new(3*i, 3*i + 1, 3*i + 2, nil, nil)
  end
  geometry.compute_face_normals
  face_color = nil
  geometry.faces.each_with_index do |face, i|
    face_color ||= Mittsu::Color.new(face.normal.x / 2.0 + 0.5, face.normal.y / 2.0 + 0.5, face.normal.z / 2.0 + 0.5)
    uv_a = calc_uvs(triangulated_side[face.a], offset, face.normal)
    uv_b = calc_uvs(triangulated_side[face.b], offset, face.normal)
    uv_c = calc_uvs(triangulated_side[face.c], offset, face.normal)
    geometry.face_vertex_uvs[0][i] = [uv_a, uv_b, uv_c]
  end
  return geometry, face_color, offset
end

def calc_uvs(vertex, offset, normal)
  u = 0.0
  v = 0.0
  if normal.x.abs <= normal.y.abs
    u = (vertex[0] + offset[0]) / TEXTURE_SCALE
    if normal.z.abs <= normal.y.abs
      v = (vertex[2] + offset[2]) / TEXTURE_SCALE
    else
      v = (vertex[1] + offset[1]) / TEXTURE_SCALE
    end
  else
    u = (vertex[1] + offset[1]) / TEXTURE_SCALE
    v = (vertex[2] + offset[2]) / TEXTURE_SCALE
  end

  Mittsu::Vector2.new(u, v)
end

texture = Mittsu::ImageUtils.load_texture(File.join(File.dirname(__FILE__), 'wall_texture.png'))
texture.wrap_s = Mittsu::RepeatWrapping
texture.wrap_t = Mittsu::RepeatWrapping
texture.mag_filter = Mittsu::NearestFilter
sky_material = Mittsu::MeshBasicMaterial.new(color: 0x00ffff)

meshes = []
sky_sides.each do |side|
  geometry, _, offset = construct_geometry_from_side(side)
  mesh_side = Mittsu::Mesh.new(geometry, sky_material)
  mesh_side.position = Mittsu::Vector3.new(offset[0], offset[1], offset[2])
  meshes << mesh_side
end
solid_sides.each do |side|
  geometry, color, offset = construct_geometry_from_side(side)
  solid_material = Mittsu::MeshBasicMaterial.new(map: texture, color: color)
  mesh_side = Mittsu::Mesh.new(geometry, solid_material)
  mesh_side.position = Mittsu::Vector3.new(offset[0], offset[1], offset[2])
  meshes << mesh_side
end

# Spline code
spline_meshes = []

def lerp(p1, p2, t)
  p1.zip(p2).map {|p| p[0] * (1 - t) + p[1] * t }
end

def get_bezier_point(bezier_points, t)
  p11 = lerp(bezier_points[0], bezier_points[1], t)
  p12 = lerp(bezier_points[1], bezier_points[2], t)
  p13 = lerp(bezier_points[2], bezier_points[3], t)

  p21 = lerp(p11, p12, t)
  p22 = lerp(p12, p13, t)

  final = lerp(p21, p22, t)
  return final
end

def sample_spline(spline)
  # Get control points
  control_points = spline.map do |s|
    xy_scale = Math.cos(s[4])
    x_offset = Math.cos(s[3]) * xy_scale * s[5]
    y_offset = Math.sin(s[3]) * xy_scale * s[5]
    z_offset = Math.sin(s[4]) * s[5]
    [[s[0], s[1], s[2]], [x_offset, y_offset, z_offset]]
  end

  (0...(control_points.length - 1)).each do |i|
    control_points[i] << control_points[i + 1][0].zip(control_points[i+1][1]).map {|e| e[0] - e[1] }
    control_points[i] << control_points[i + 1][0]
    control_points[i][1] = control_points[i][0].zip(control_points[i][1]).map {|e| e[0] + e[1] }
  end
  control_points = control_points[0...-1]

  return [] if control_points.length == 0

  # Sample spline
  sampled_points = []
  control_points.each do |bezier_points|
    (0...SAMPLES_PER_SEGMENT).each do |i|
      t = i.to_f / SAMPLES_PER_SEGMENT
      p = get_bezier_point(bezier_points, t)
      sampled_points << p
    end
  end
  sampled_points << control_points[-1][-1]
end

def regenerate_spline(spline, spline_meshes, current_point)
  return if spline.length <= 1

  sampled_points = sample_spline(spline).map {|p| Mittsu::Vector3.new(p[0], p[1], p[2]) }

  # Construct meshes
  spline_meshes = spline_meshes.clear
  (0...(sampled_points.length - 1)).each do |i|
    distance = Math.sqrt(sampled_points[i].distance_to_squared(sampled_points[i+1]))
    geometry = Mittsu::BoxGeometry.new(4.0, 4.0, distance)

    if i == current_point * SAMPLES_PER_SEGMENT or i == current_point * SAMPLES_PER_SEGMENT - 1
      material = Mittsu::MeshBasicMaterial.new(color: 0xffff00)
    elsif i >= (current_point - 1) * SAMPLES_PER_SEGMENT and i < (current_point + 1) * SAMPLES_PER_SEGMENT
      material = Mittsu::MeshBasicMaterial.new(color: 0xff00ff)
    else
      material = Mittsu::MeshBasicMaterial.new(color: 0xffffff)
    end

    mesh = Mittsu::Mesh.new(geometry, material)
    mesh.position = sampled_points[i]
    mesh.look_at(sampled_points[i+1])
    mesh.translate_on_axis(Mittsu::Vector3.new(0.0, 0.0, 1.0), distance / 2)
    spline_meshes << mesh
  end
end

def snap(value, snap_value)
  (value / snap_value).round * snap_value
end

if save_loaded
  regenerate_spline(spline, spline_meshes, current_point)
end

# Spline export
def export_spline(spline)
  sampled_points = sample_spline(spline)
  File.open("spline_output.txt", "w") do |f|
    sampled_points.each do |s|
      f.write("setpos #{s[0]} #{s[1]} #{s[2]};setang 0 0 0\n")
    end
  end
end

# Start render loop
renderer.window.set_mouselock(true)
frame = 0
y_center = 0.0
recull = true
spacebar_oneshot = true
left_oneshot = true
right_oneshot = true
up_oneshot = true
down_oneshot = true
export_oneshot = true
renderer.window.run do
  # Cull objects that are too far to speed up the rendering process
  if frame % CULL_UPDATE_RATE == 0 or recull
    scene.children = []
    (meshes + spline_meshes).each do |m|
      diff = camera.get_world_position.sub(m.get_world_position)
      if diff.x.abs + diff.y.abs + diff.z.abs < RENDER_DISTANCE * CULL_FACTOR
        scene.add(m)
      end
    end
    recull = false
  end

  if renderer.window.key_down?(GLFW_KEY_ESCAPE)
    renderer.window.set_mouselock(false)
  end
  if renderer.window.mouse_button_down?(GLFW_MOUSE_BUTTON_LEFT)
    renderer.window.set_mouselock(true)
  end

  # Set Camera Angle
  mouse_pos = renderer.window.mouse_position
  mouse_pos[0] -= SCREEN_WIDTH / 2
  mouse_pos[1] -= SCREEN_HEIGHT / 2

  if mouse_pos[1] - y_center > PIXELS_PER_180 / 2.0
    y_center = mouse_pos[1] - PIXELS_PER_180 / 2.0
  elsif y_center - mouse_pos[1] > PIXELS_PER_180 / 2.0
    y_center = mouse_pos[1] + PIXELS_PER_180 / 2.0
  end

  xy_angle = (-mouse_pos[0] / PIXELS_PER_180).modulo(2.0) * Math::PI
  z_angle = (y_center - mouse_pos[1]) / PIXELS_PER_180 * Math::PI

  camera.set_rotation_from_axis_angle(Mittsu::Vector3.new(0.0, 0.0, 1.0), xy_angle - Math::PI / 2.0)
  camera.rotate_x(Math::PI / 2.0 + z_angle)

  # Noclip Controls
  if renderer.window.key_down?(GLFW_KEY_W)
    camera.translate_on_axis(Mittsu::Vector3.new(0.0, 0.0, 1.0), -NOCLIP_SPEED)
  end
  if renderer.window.key_down?(GLFW_KEY_S)
    camera.translate_on_axis(Mittsu::Vector3.new(0.0, 0.0, 1.0), NOCLIP_SPEED)
  end
  if renderer.window.key_down?(GLFW_KEY_A)
    camera.translate_on_axis(Mittsu::Vector3.new(1.0, 0.0, 0.0), -NOCLIP_SPEED)
  end
  if renderer.window.key_down?(GLFW_KEY_D)
    camera.translate_on_axis(Mittsu::Vector3.new(1.0, 0.0, 0.0), NOCLIP_SPEED)
  end

  # Spline Controls
  if renderer.window.key_down?(GLFW_KEY_SPACE) and spacebar_oneshot
    new_point = camera.get_world_position.elements.map {|v| snap(v, GRID_SNAP) }
    new_point << snap(xy_angle, ANGLE_SNAP_RAD)
    new_point << snap(z_angle, ANGLE_SNAP_RAD)
    new_point << spline_length
    spline[current_point] = new_point

    current_point += 1
    regenerate_spline(spline, spline_meshes, current_point)
    recull = true
    spacebar_oneshot = false
  elsif not renderer.window.key_down?(GLFW_KEY_SPACE)
    spacebar_oneshot = true
  end

  if renderer.window.key_down?(GLFW_KEY_LEFT) and left_oneshot
    current_point = [current_point - 1, 0].max
    spline_length = spline[current_point][5]
    regenerate_spline(spline, spline_meshes, current_point)
    puts "Current spline bend-length: #{spline_length.round(3)}"
    recull = true
    left_oneshot = false
  elsif not renderer.window.key_down?(GLFW_KEY_LEFT)
    left_oneshot = true
  end
  if renderer.window.key_down?(GLFW_KEY_RIGHT) and right_oneshot
    current_point = [current_point + 1, spline.length].min
    spline_length = spline[current_point][5] if current_point != spline.length
    regenerate_spline(spline, spline_meshes, current_point)
    puts "Current spline bend-length: #{spline_length.round(3)}"
    recull = true
    right_oneshot = false
  elsif not renderer.window.key_down?(GLFW_KEY_RIGHT)
    right_oneshot = true
  end

  if renderer.window.key_down?(GLFW_KEY_UP) and up_oneshot
    spline_length += SPLINE_LENGTH_INCREMENT
    spline[current_point][5] = spline_length if current_point != spline.length
    regenerate_spline(spline, spline_meshes, current_point)
    puts "Current spline bend-length: #{spline_length.round(3)}"
    recull = true
    up_oneshot = false
  elsif not renderer.window.key_down?(GLFW_KEY_UP)
    up_oneshot = true
  end
  if renderer.window.key_down?(GLFW_KEY_DOWN) and down_oneshot
    spline_length = [spline_length - SPLINE_LENGTH_INCREMENT, 0.0].max
    spline[current_point][5] = spline_length if current_point != spline.length
    regenerate_spline(spline, spline_meshes, current_point)
    puts "Current spline bend-length: #{spline_length.round(3)}"
    recull = true
    down_oneshot = false
  elsif not renderer.window.key_down?(GLFW_KEY_DOWN)
    down_oneshot = true
  end

  # Spline Save
  if renderer.window.key_down?(GLFW_KEY_F) and export_oneshot
    export_spline(spline)
    export_oneshot = false
  elsif not renderer.window.key_down?(GLFW_KEY_F)
    export_oneshot = true
  end

  renderer.render(scene, camera)
  frame += 1
end

return if spline == []

file_end = 1
file_end += 1 while File.file?("spline_save_%04d.txt" % file_end)
File.open("spline_save_%04d.txt" % file_end, "w") do |f|
  spline.each do |s|
    s[3] *= 180.0 / Math::PI
    s[4] *= 180.0 / Math::PI
    f.write(s.map {|v| v.round(5) }.join(" ") + "\n")
  end
end
