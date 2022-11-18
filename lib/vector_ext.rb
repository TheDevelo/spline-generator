require 'matrix'

class Vector
  def project_plane(normal)
    self - (self.dot normal) * normal
  end

  def rotate_around(normal, angle)
    parallel = (self.dot normal) * normal
    orthogonal = self - parallel
    cross = normal.cross_product orthogonal
    parallel + Math.cos(angle) * orthogonal + Math.sin(angle) * cross
  end
end
