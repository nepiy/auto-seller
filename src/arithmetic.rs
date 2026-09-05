use crate::error::{BotError, Result};
use alloy::primitives::U256;

/// Multiply integers by the exact configured floating-point ratio and round
/// upward, without first losing integer precision through an f64 conversion.
pub(crate) fn scale_u128(value: u128, multiplier: f64) -> Result<u128> {
    if !multiplier.is_finite() || multiplier < 1.0 {
        return Err(BotError::Config(
            "fee/gas multiplier must be finite and at least one".into(),
        ));
    }
    if value == 0 {
        return Ok(0);
    }
    let bits = multiplier.to_bits();
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023 - 52;
    let product = U256::from(value) * U256::from(mantissa);
    let result = if exponent < 0 {
        product.div_ceil(U256::from(1) << (-exponent as usize))
    } else {
        let shift = exponent as usize;
        if shift >= 128 || product.bit_len() + shift > 128 {
            return Err(BotError::Transaction("scaled fee overflowed u128".into()));
        }
        product << shift
    };
    result
        .try_into()
        .map_err(|_| BotError::Transaction("scaled fee overflowed u128".into()))
}

pub(crate) fn scale_u64(value: u64, multiplier: f64) -> Result<u64> {
    scale_u128(u128::from(value), multiplier)?
        .try_into()
        .map_err(|_| BotError::Transaction("scaled gas limit overflowed u64".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_preserves_large_integers_and_rejects_overflow() {
        assert_eq!(scale_u128(u128::MAX, 1.0).unwrap(), u128::MAX);
        assert_eq!(scale_u64(u64::MAX, 1.0).unwrap(), u64::MAX);
        assert_eq!(
            scale_u128((1u128 << 100) + 1, 1.0).unwrap(),
            (1u128 << 100) + 1
        );
        assert!(scale_u128(u128::MAX, 1.0000000000000002).is_err());
        assert!(scale_u64(u64::MAX, 2.0).is_err());
        assert!(scale_u128(1, f64::MAX).is_err());
        assert_eq!(scale_u128(1, 1.15).unwrap(), 2);
    }
}
