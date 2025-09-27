pub const PI: f32 = 3.14159265358979323846;
pub const PI_2: f32 = 2.0 * PI;
pub const HALF_PI: f32 = 0.5 * PI;
pub const QUARTER_PI: f32 = 0.25 * PI;
pub const ONE_OVER_PI: f32 = 1.0 / PI;
pub const ONE_OVER_TWO_PI: f32 = 1.0 / PI_2;
pub const SQRT_TWO: f32 = 1.41421356237309504880;
pub const SQRT_THREE: f32 = 1.73205080756887729352;
pub const SQRT_ONE_OVER_TWO: f32 = 0.70710678118654752440;
pub const SQRT_ONE_OVER_THREE: f32 = 0.57735026918962576450;
pub const DEG2RAD_MULTIPLIER: f32 = PI / 180.0;
pub const RAD2DEG_MULTIPLIER: f32 = 180.0 / PI;
pub const FLOAT_EPSILON: f32 = 1.192092896e-07;
pub const INFINITY: f32 = 1e30;
pub const SEC_TO_MS_MULTIPLIER: f32 = 1000.0;
pub const MS_TO_SEC_MULTIPLIER: f32 = 0.001;
pub const INVALID_ID: usize = usize::MAX;

pub fn deg_to_rad(degrees: f32) -> f32 {
    degrees * DEG2RAD_MULTIPLIER
}

pub fn rad_to_deg(rad: f32) -> f32 {
    rad * RAD2DEG_MULTIPLIER
}
