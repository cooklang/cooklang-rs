/// Absorbs representation error near zero, where a relative tolerance vanishes.
const ABS_TOLERANCE: f64 = 1e-5;

/// Absorbs the error of converting between units. The bundled imperial ratios
/// in `units.toml` are truncated at nine decimals, so canonical equivalences
/// disagree once converted to the metric base unit: 48 tsp against 1 cup is
/// the worst at 1.2e-7 of its size. A difference a recipe writes down is at
/// least 1e-4 of its size, so this sits between the two with margin on both
/// sides. Relative, because the base unit is the smallest of the family (mg,
/// ml): an absolute tolerance there is a different looseness for every family
/// and every magnitude.
const REL_TOLERANCE: f64 = 1e-6;

pub(crate) fn equal_f64(a: f64, b: f64) -> bool {
    equal_f64_with_tolerance(a, b, ABS_TOLERANCE)
        || equal_f64_with_tolerance(a, b, REL_TOLERANCE * a.abs().max(b.abs()))
}

pub(crate) fn equal_f64_with_tolerance(a: f64, b: f64, epsilon: f64) -> bool {
    // Covers exactly equal (with epsilon 0)
    if a == b {
        return true;
    }
    (a - b).abs() < epsilon
}

/// Removes the error f64 arithmetic leaves behind without touching any digit a
/// recipe could have written: `3.3000000000000003` is `3.3` and
/// `550.0000000000001` is `550`, while `2.5000025` is kept. Twelve significant
/// digits, because a recipe writes a handful and f64 carries about sixteen.
pub(crate) fn snap_f64(n: f64) -> f64 {
    if n == 0.0 || !n.is_finite() {
        return n;
    }
    let magnitude = n.abs().log10().floor() as i32;
    // far beyond any amount a recipe holds the scaling itself loses bits, so
    // such a value is left as it is
    if magnitude.abs() > 100 {
        return n;
    }
    let factor = 10f64.powi(12 - magnitude);
    (n * factor).round() / factor
}

pub(crate) fn round_f64(val: f64, precision: u32) -> f64 {
    let factor = 10.0_f64.powi(precision as i32);
    (val * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.01 is wide enough that a case reads as "about a hundredth apart"
    const EPS: f64 = 0.01;

    #[track_caller]
    fn assert_within(a: f64, b: f64, epsilon: f64) {
        assert!(
            equal_f64_with_tolerance(a, b, epsilon),
            "  epsilon: {epsilon}\n  got: {a} != {b}\n  expected: {a} == {b}"
        );
    }

    #[track_caller]
    fn assert_not_within(a: f64, b: f64, epsilon: f64) {
        assert!(
            !equal_f64_with_tolerance(a, b, epsilon),
            "  epsilon: {epsilon}\n  got: {a} == {b}\n  expected: {a} != {b}"
        );
    }

    #[track_caller]
    fn assert_equal(a: f64, b: f64) {
        assert!(equal_f64(a, b), "  got: {a} != {b}\n  expected: {a} == {b}");
    }

    #[track_caller]
    fn assert_not_equal(a: f64, b: f64) {
        assert!(
            !equal_f64(a, b),
            "  got: {a} == {b}\n  expected: {a} != {b}"
        );
    }

    #[test]
    fn compare_floats_within_a_given_tolerance() {
        // the same value, however it is written
        assert_within(2.0, 2.0, EPS);
        assert_within(0.0, -0.0, EPS);
        assert_not_within(2.0, 3.0, EPS);

        // a difference smaller than the tolerance
        assert_within(1.0, 1.005, EPS);
        assert_not_within(1.0, 1.02, EPS);
        // the tolerance is absolute, so the same slack applies at every
        // magnitude: it neither widens at a hundred nor narrows at a tenth
        assert_within(100.0, 100.001, EPS);
        assert_not_within(100.0, 100.1, EPS);
        assert_within(0.1, 0.105, EPS);
        // and the order of the arguments cannot change the answer
        assert_not_within(10.0, 10.1, EPS);
        assert_not_within(10.1, 10.0, EPS);

        // a difference of exactly the tolerance is not within it
        assert_not_within(0.0, 0.01, EPS);

        // an absolute tolerance absorbs anything near enough to zero
        assert_within(0.0, 0.000_001, EPS);
        // opposite signs are two values apart, not none
        assert_not_within(1.0, -1.0, EPS);

        // a zero tolerance asks for exactly the same value
        assert_within(1.0, 1.0, 0.0);
        assert_not_within(1.0, 1.000_000_1, 0.0);

        // an infinity equals only itself, and NaN equals nothing, because no
        // tolerance can span an infinite difference
        assert_within(f64::INFINITY, f64::INFINITY, EPS);
        assert_not_within(f64::INFINITY, 1.0, EPS);
        assert_not_within(f64::NEG_INFINITY, f64::INFINITY, EPS);
        assert_not_within(f64::NAN, f64::NAN, EPS);
    }

    #[test]
    fn the_default_tolerance_absorbs_error_but_not_amounts() {
        // the error left behind when a value cannot be represented exactly
        assert_equal(0.1 + 0.2, 0.3);
        // and the residue of converting between units, in millilitres: 9 tsp
        // against 3 tbsp, then 48 tsp against 1 cup, the worst canonical pair
        assert_equal(44.360_289, 44.360_292);
        assert_equal(236.588_208, 236.588_236);
        // the slack scales with the amount: the same conversion error a
        // thousand times larger is still absorbed, and a written difference of
        // the same size is not
        assert_equal(236_588.208, 236_588.236);
        assert_not_equal(236_588.0, 236_589.0);

        // but not a difference a recipe would write down
        assert_not_equal(1.0, 1.000_1);
        assert_not_equal(44.3, 44.4);
    }

    #[test]
    fn snap_removes_float_noise_and_nothing_else() {
        assert_eq!(snap_f64(3.0 * 1.1), 3.3);
        assert_eq!(snap_f64(500.0 * 1.1), 550.0);
        assert_eq!(snap_f64(0.1 + 0.2), 0.3);
        // a digit a recipe could have written is not noise
        assert_eq!(snap_f64(2.500_002_5), 2.500_002_5);
        assert_eq!(snap_f64(1.000_1), 1.000_1);
        // nothing to snap
        assert_eq!(snap_f64(0.0), 0.0);
        assert_eq!(snap_f64(-4.5), -4.5);
        assert!(snap_f64(f64::NAN).is_nan());
        assert_eq!(snap_f64(f64::INFINITY), f64::INFINITY);
        // an amount far beyond any recipe comes back unchanged
        assert_eq!(snap_f64(1e-300), 1e-300);
        assert_eq!(snap_f64(1e300), 1e300);
    }

    #[test]
    fn round_floats() {
        // 3.0 * 1.1 is 3.3000000000000003, so rounding is what makes a scaled
        // value printable again
        assert_eq!(round_f64(3.0 * 1.1, 1), 3.3);
        assert_eq!(round_f64(1.0 / 3.0, 2), 0.33);
        assert_eq!(round_f64(2.0 / 3.0, 2), 0.67);

        // a precision of zero rounds to a whole number, halves away from zero
        assert_eq!(round_f64(3.3, 0), 3.0);
        assert_eq!(round_f64(4.5, 0), 5.0);
        assert_eq!(round_f64(-4.5, 0), -5.0);

        // a value already shorter than the precision asked for is untouched
        assert_eq!(round_f64(2.5, 3), 2.5);
    }
}
