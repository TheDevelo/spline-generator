require 'optparse'

require 'file_parser'
require 'model'
require 'texture'

class OptionError < StandardError
end

# Parse CMD options
params = {}
opt_parser = OptionParser.new do |opts|
  opts.banner = "Usage: [options] INPUT_FILE"

  opts.accept(Texture::Color) do |hex|
    begin
      Texture::Color.new(hex)
    rescue
      raise OptionError.new "Color does not match format of #XXXXXX"
    end
  end

  opts.on("-c", "--color COLOR", "Change the color of the model", Texture::Color)
  opts.on("-n", "--name NAME", "Name of the model after compilation")
  opts.on("-p", "--prisms PRISMS", "Number of connecting prisms between vertices per model", Integer) do |o|
    if o >= 1
      o
    else
      raise OptionError.new "Number of prisms per model must be at least 1"
    end
  end
  opts.on("-r", "--radius RADIUS", "Radius of the model's prisms", Float)
  opts.on("-s", "--sides SIDES", "Number of sides on the model's prisms", Integer) do |o|
    if o >= 2
      o
    else
      raise OptionError.new "Sides must be at least 2"
    end
  end
end
opt_parser.parse!(into: params)

# Set defaults to parameters not passed in
params[:color] = Texture::Color.new "#ffffff" unless params.include? :color
params[:name] = "bp-gen/botpath" unless params.include? :name
params[:prisms] = nil unless params.include? :prisms
params[:radius] = 4.0 unless params.include? :radius
params[:sides] = 6 unless params.include? :sides

if ARGV.length != 1
  puts opt_parser.help
  return
end

if not File.file?(ARGV[0])
  puts "ERROR: Pass in a valid input file"
  puts opt_parser.help
  return
end

verts = FileParser.parse_log(File.read(ARGV[0]))
vtf, vmt = Texture.generate_unlit_color(params[:color], "bp-gen/botpath")
model_pairs = Model.generate_model(verts, params[:name], params[:radius], "botpath-#{params[:color].to_s}", "bp-gen", params[:sides], params[:prisms])

# Write files
model_pairs.each_with_index do |pair, i|
  File.write("botpath_sec#{i+1}.smd", pair[0])
  File.write("botpath_sec#{i+1}.qc", pair[1])
end
Dir.mkdir("materials/") unless Dir.exist?("materials/")
Dir.mkdir("materials/bp-gen/") unless Dir.exist?("materials/bp-gen/")
File.write("materials/bp-gen/botpath.vtf", vtf)
File.write("materials/bp-gen/botpath-#{params[:color].to_s}.vmt", vmt)
