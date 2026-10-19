//! 3D attributes, distance attenuation, Doppler.

/// World-space position / velocity (right-handed, units = meters).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

/// Emitter or listener 3D state.
#[derive(Debug, Clone, Copy)]
pub struct Attributes3d {
    pub position: Vec3,
    pub velocity: Vec3,
    /// Forward unit vector (listener only for orientation; emitters optional).
    pub forward: Vec3,
    pub up: Vec3,
}

impl Default for Attributes3d {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            forward: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
        }
    }
}

/// Distance attenuation curve (linear roll-off between min and max).
#[derive(Debug, Clone, Copy)]
pub struct Attenuation {
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for Attenuation {
    fn default() -> Self {
        Self {
            min_distance: 1.0,
            max_distance: 50.0,
        }
    }
}

impl Attenuation {
    /// Linear gain 1.0 at ≤ min, 0.0 at ≥ max.
    pub fn gain_at(&self, distance: f32) -> f32 {
        if distance <= self.min_distance {
            return 1.0;
        }
        if distance >= self.max_distance {
            return 0.0;
        }
        let t = (distance - self.min_distance) / (self.max_distance - self.min_distance).max(1e-6);
        1.0 - t
    }
}

/// Speed of sound (m/s) for Doppler.
pub const SPEED_OF_SOUND: f32 = 343.0;

/// Doppler pitch scale from relative radial velocity.
///
/// Positive `closing_speed` (source moving toward listener) raises pitch.
pub fn doppler_pitch(closing_speed: f32) -> f32 {
    let c = SPEED_OF_SOUND;
    let v = closing_speed.clamp(-c * 0.9, c * 0.9);
    (c / (c - v)).clamp(0.25, 4.0)
}

/// Radial component of relative velocity along the listener→emitter axis.
pub fn closing_speed(listener: &Attributes3d, emitter: &Attributes3d) -> f32 {
    let to_src = emitter.position.sub(listener.position);
    let dist = to_src.length().max(1e-4);
    let dir = Vec3::new(to_src.x / dist, to_src.y / dist, to_src.z / dist);
    // Relative velocity of source wrt listener projected onto direction to source.
    let rel = emitter.velocity.sub(listener.velocity);
    -rel.dot(dir) // toward listener = positive closing
}

pub fn distance(listener: &Attributes3d, emitter: &Attributes3d) -> f32 {
    emitter.position.sub(listener.position).length()
}
