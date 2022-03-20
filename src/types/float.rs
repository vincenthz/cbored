/// CBOR Float (FP16, FP32, FP64)
#[derive(Clone, Debug, Copy)]
pub enum Float {
    /// Half Precision IEEE754 (2 bytes)
    FP16(u16),
    /// Normal Precision IEEE754 (4 bytes)
    FP32(u32),
    /// Double Precision IEEE754 (8 bytes)
    FP64(u64),
}

impl Float {
    pub fn to_f32(&self) -> f32 {
        match self {
            Float::FP16(fp) => f32::from_bits(ieee754_u16_to_u32(*fp)),
            Float::FP32(fp) => f32::from_bits(*fp),
            Float::FP64(fp) => f64::from_bits(*fp) as f32,
        }
    }
    pub fn to_f64(&self) -> f64 {
        match self {
            Float::FP16(fp) => f32::from_bits(ieee754_u16_to_u32(*fp)).into(),
            Float::FP32(fp) => f32::from_bits(*fp).into(),
            Float::FP64(fp) => f64::from_bits(*fp),
        }
    }
}

// convert a u16 holding a IEEE754 FP16 to a u32 representing a IEEE754 FP32
//
// FP16:
// sign (1 bit), exponent (5 bits), fraction (10 bits)
//
// FP32:
// sign (1 bit), exponent (8 bits), fraction (23 bits)
fn ieee754_u16_to_u32(v: u16) -> u32 {
    const F32_EXPONENT: usize = 23;
    const F16_EXPONENT: usize = 10;
    const F32_EXP_BIAS: u32 = 127;
    const F16_EXP_BIAS: u32 = 15;

    // handle 0
    if v & 0x7FFF == 0 {
        return (v as u32) << 16;
    }

    let fp16sign = (v & 0b1000_0000_0000_0000) as u32;
    let fp16exp = (v & 0b0111_1100_0000_0000) as u32;
    let fp16frac = (v & 0b0000_0011_1111_1111) as u32;

    let sign = fp16sign << 16;

    // Infinity and NaN
    if fp16exp == 0x7C00 {
        if fp16frac == 0 {
            sign | 0x7F80_0000
        } else {
            sign | 0x7FC0_0000 | (fp16frac << 13)
        }
    // Check for subnormals
    } else if fp16exp == 0 {
        // Calculate how much to adjust the exponent by and adjust exponent
        let e = (fp16frac as u16).leading_zeros() - 6;
        let exp = (F32_EXP_BIAS - F16_EXP_BIAS - e) << F32_EXPONENT;
        let frac = (fp16frac << (14 + e)) & 0x007F_FFFF;
        sign | exp | frac
    } else {
        // change exponent from bias 15 starting at bit 10, to bias 127 at bit 23
        let unbiased_exp = ((fp16exp >> F16_EXPONENT) as i32) - F16_EXP_BIAS as i32;
        let exp = ((unbiased_exp + F32_EXP_BIAS as i32) as u32) << F32_EXPONENT;
        sign | exp | (fp16frac << 13)
    }
}
