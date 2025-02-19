// Crop Coefficients struct to hold the mean coefficients for each crop stage using growing degree days, it contains the length of the
// period and the end Kc for each stage, ensure that if you are using Fahrenheit GDD coefficients, then the cumulative GDD should be in Fahrenheit.
pub struct CropCoefficientsGdd {
    crop_name: String,
    initial_end_kc: (f32, f32),
    development_end_kc: (f32, f32),
    mid_end_kc: (f32, f32),
    late_end_kc: (f32, f32),
}

impl CropCoefficientsGdd {
    /// Creates a new instance of `CropCoefficients` with specified parameters for each growth stage.
    ///
    /// # Parameters
    ///
    /// - `crop_name`: A `String` representing the name of the crop.
    /// - `initial_end_kc`: A tuple `(f32, f32)` representing the length of the initial growth period in cumulative GDD and the end mean Kc value for this stage.
    /// - `development_end_kc`: A tuple `(f32, f32)` representing the length of the development growth period in cumulative GDD and the end mean Kc value for this stage.
    /// - `mid_end_kc`: A tuple `(f32, f32)` representing the length of the mid-season growth period in cumulative GDD and the end mean Kc value for this stage.
    /// - `late_end_kc`: A tuple `(f32, f32)` representing the length of the late growth period in cumulative GDD and the end mean Kc value for this stage.
    ///
    /// # Returns
    ///
    /// A `CropCoefficients` struct initialized with the provided parameters. Panics if any length of period is not positive or if any Kc value exceeds 2.
    pub fn new(crop_name: String, initial_end_kc: (f32, f32), development_end_kc: (f32, f32), mid_end_kc: (f32, f32), late_end_kc: (f32, f32)) -> CropCoefficientsGdd {
        // check if length of period is positive
        if initial_end_kc.0 < 0.0 || development_end_kc.0 < 0.0 || mid_end_kc.0 < 0.0 || late_end_kc.0 < 0.0 {
            panic!("Length of period must be positive.");
        }

        // check if Kc cannot exceed 2
        if initial_end_kc.1 > 2.0 || development_end_kc.1 > 2.0 || mid_end_kc.1 > 2.0 || late_end_kc.1 > 2.0 {
            panic!("Kc cannot exceed 2.");
        }

        CropCoefficientsGdd {
            crop_name,
            initial_end_kc,
            development_end_kc,
            mid_end_kc,
            late_end_kc,
        }
    }
}

/// Calculates the crop coefficient (Kc) based on the cumulative growing degree days (GDD) and other optional environmental factors.
/// It will adjust the Kc for wind speed, relative humidity, and crop height if provided the optional environmental factors.
///
/// # Parameters
///
/// - `cumulative_gdd`: A `f32` representing the cumulative growing degree days, which is used to determine the crop growth stage.
/// - `cc`: A `CropCoefficients` struct containing the crop coefficients for different growth stages.
/// - `wind_speed`: An `Option<f32>` representing the wind speed in m/s. If not provided, defaults to 2.0 m/s.
/// - `rh_min`: An `Option<f32>` representing the minimum relative humidity in percentage. If not provided, defaults to 45.0%.
/// - `crop_height`: An `Option<f32>` representing the crop height in meters. If not provided, defaults to 0.0 m.
///
/// # Returns
///
/// A `(String, f32)` representing the name of the corp and the calculated crop coefficient (Kc) adjusted if given environmental conditions.
pub fn crop_coefficient_gdd(cumulative_gdd: f32, cc: CropCoefficientsGdd, wind_speed: Option<f32>, rh_min: Option<f32>, crop_height: Option<f32>) -> (String, f32) {
    let wind_speed = wind_speed.unwrap_or(2.0);
    let mut rh_min = rh_min.unwrap_or(45.0);
    let crop_height = crop_height.unwrap_or(1.391);

    if rh_min < 1.0 {
        rh_min *= 100.0; // Convert to percentage
    }

    if cumulative_gdd <= cc.initial_end_kc.0 {
        (cc.crop_name, (cc.initial_end_kc.1 * 100.0).round() / 100.0) // Kc for initial stage
    } else if cumulative_gdd <= cc.development_end_kc.0 {
        // Interpolation between initial and development stages
        (cc.crop_name, ((cc.initial_end_kc.1 + (cc.development_end_kc.1 - cc.initial_end_kc.1) * ((cumulative_gdd - cc.initial_end_kc.0) / (cc.development_end_kc.0 - cc.initial_end_kc.0))) * 100.0) / 100.0) // Kc for development stage
    } else if cumulative_gdd <= cc.mid_end_kc.0 {
        // Interpolation between development and mid-season stages
        let kc_org = cc.development_end_kc.1 + (cc.mid_end_kc.1 - cc.development_end_kc.1) * ((cumulative_gdd - cc.development_end_kc.0) / (cc.mid_end_kc.0 - cc.development_end_kc.0));
        // Adjust Kc based on crop height and wind speed to compensate for arid and windy conditions
        (cc.crop_name, (adjust_kc(kc_org, wind_speed, rh_min, crop_height) * 100.0) / 100.0) // Kc for mid-season stage
    } else {
        // Interpolation between mid-season and end stages
        let kc_org = cc.mid_end_kc.1 - (cc.mid_end_kc.1 - cc.late_end_kc.1) * ((cumulative_gdd - cc.late_end_kc.0) / (cc.mid_end_kc.0 - cc.late_end_kc.0));
        // let kc_org = (cc.development_end_kc.1 - ((cumulative_gdd - cc.mid_end_kc.0) / cc.late_end_kc.0)).max(cc.late_end_kc.1);
        // Adjust Kc based on crop height and wind speed if it's larger than 0.45
        if kc_org > 0.45 {
            (cc.crop_name, (adjust_kc(kc_org, wind_speed, rh_min, crop_height) * 100.0) / 100.0) // Kc for end stage
        } else {
            (cc.crop_name, (kc_org * 100.0) / 100.0) // Kc for end stage with default adjustment if Kc is less than 0.45
        }
    }
}

pub(crate) fn adjust_kc(kc_original: f32, wind_speed: f32, rh_min: f32, crop_height: f32) -> f32 {
    let term1 = 0.04 * (wind_speed - 2.0);
    let term2 = 0.0004 * (rh_min - 45.0);
    let adjustment = (term1 - term2) * (crop_height / 3.0).powf(0.3);
    kc_original + adjustment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Should return the initial stage Kc when cumulative GDD is exactly at the initial_end_kc threshold
    fn test_crop_coefficient_gdd_initial_stage() {
        let crop_name = "Wheat".to_string();
        let initial_end_kc = (100.0, 0.3);
        let development_end_kc = (200.0, 0.5);
        let mid_end_kc = (300.0, 0.8);
        let late_end_kc = (400.0, 0.6);

        let cc = CropCoefficientsGdd::new(
            crop_name.clone(),
            initial_end_kc,
            development_end_kc,
            mid_end_kc,
            late_end_kc,
        );

        let cumulative_gdd = 100.0;
        let (name, kc) = crop_coefficient_gdd(cumulative_gdd, cc, None, None, None);

        assert_eq!(name, crop_name);
        assert!((kc - 0.3).abs() < 0.001);
    }

    #[test]
    // Should interpolate correctly between initial and development stages when cumulative GDD is between initial_end_kc and development_end_kc
    fn test_crop_coefficient_gdd_interpolation_initial_to_development() {
        let crop_name = "Test Crop".to_string();
        let initial_end_kc = (100.0, 0.5);
        let development_end_kc = (200.0, 1.0);
        let mid_end_kc = (300.0, 1.2);
        let late_end_kc = (400.0, 0.8);

        let crop_coefficients = CropCoefficientsGdd::new(
            crop_name.clone(),
            initial_end_kc,
            development_end_kc,
            mid_end_kc,
            late_end_kc,
        );

        let cumulative_gdd = 150.0; // Between initial_end_kc and development_end_kc
        let expected_kc = 0.75; // Linear interpolation between 0.5 and 1.0

        let (result_crop_name, result_kc) = crop_coefficient_gdd(cumulative_gdd, crop_coefficients, None, None, None);

        assert_eq!(result_crop_name, crop_name);
        assert!((result_kc - expected_kc).abs() < 0.01);
    }

    #[test]
    // Should interpolate correctly between development and mid-season stages when cumulative GDD is between development_end_kc and mid_end_kc
    fn test_interpolation_between_development_and_mid_season_stages() {
        let crop_coefficients = CropCoefficientsGdd::new(
            "TestCrop".to_string(),
            (100.0, 0.5),  // initial_end_kc
            (200.0, 0.8),  // development_end_kc
            (300.0, 1.2),  // mid_end_kc
            (400.0, 0.7),  // late_end_kc
        );

        let cumulative_gdd = 250.0; // Between development_end_kc and mid_end_kc
        let wind_speed = Some(2.0);
        let rh_min = Some(45.0);
        let crop_height = Some(1.0);

        let (crop_name, kc) = crop_coefficient_gdd(cumulative_gdd, crop_coefficients, wind_speed, rh_min, crop_height);

        assert_eq!(crop_name, "TestCrop");
        assert!((kc - 1.0).abs() < 0.01);
    }

    #[test]
    // Should interpolate correctly between mid-season and end stages when cumulative GDD is between mid_end_kc and late_end_kc
    fn test_crop_coefficient_gdd_interpolation_mid_to_end_stage() {
        let crop_coefficients = CropCoefficientsGdd::new(
            "TestCrop".to_string(),
            (100.0, 0.3),  // initial_end_kc
            (200.0, 0.5),  // development_end_kc
            (300.0, 1.0),  // mid_end_kc
            (400.0, 0.7),  // late_end_kc
        );

        let cumulative_gdd = 350.0; // Between mid_end_kc and late_end_kc
        // let wind_speed = Some(3.0);
        // let rh_min = Some(50.0);
        // let crop_height = Some(1.5);

        let (crop_name, kc) = crop_coefficient_gdd(cumulative_gdd, crop_coefficients, None, None, None);

        assert_eq!(crop_name, "TestCrop");
        assert!((kc - 0.85).abs() < 0.01, "Expected Kc to be interpolated correctly between mid and end stages");
    }

    #[test]
    // Should adjust Kc for mid-season stage based on wind speed, relative humidity, and crop height when environmental factors are provided
    fn test_adjust_kc_mid_season_with_environmental_factors() {
        let crop_coefficients = CropCoefficientsGdd::new(
            "Corn".to_string(),
            (200.0, 0.3),
            (500.0, 1.15),
            (800.0, 1.2),
            (1000.0, 0.5),
        );

        let cumulative_gdd = 600.0; // Mid-season stage
        let wind_speed = Some(3.0); // m/s
        let rh_min = Some(30.0); // %
        let crop_height = Some(1.5); // meters

        let (crop_name, kc) = crop_coefficient_gdd(
            cumulative_gdd,
            crop_coefficients,
            wind_speed,
            rh_min,
            crop_height,
        );

        assert_eq!(crop_name, "Corn");
        assert!((kc - 1.2).abs() < 0.01, "Expected Kc to be adjusted for mid-season stage with environmental factors");
    }

    #[test]
    // Should adjust Kc for end stage based on wind speed, relative humidity, and crop height when Kc is greater than 0.45
    fn test_adjust_kc_for_end_stage() {
        let crop_name = "Corn".to_string();
        let initial_end_kc = (100.0, 0.3);
        let development_end_kc = (200.0, 0.7);
        let mid_end_kc = (300.0, 1.2);
        let late_end_kc = (400.0, 0.5);

        let cc = CropCoefficientsGdd::new(crop_name, initial_end_kc, development_end_kc, mid_end_kc, late_end_kc);

        let cumulative_gdd = 350.0; // GDD in the end stage
        let wind_speed = Some(3.0); // m/s
        let rh_min = Some(30.0); // percentage
        let crop_height = Some(1.5); // meters

        let (crop_name_result, kc_result) = crop_coefficient_gdd(cumulative_gdd, cc, wind_speed, rh_min, crop_height);

        assert_eq!(crop_name_result, "Corn");
        assert!((kc_result - 0.887).abs() < 0.01); // Expected Kc after adjustment
    }

    #[test]
    // Should not adjust Kc for end stage when Kc is less than or equal to 0.45
    fn test_crop_coefficient_gdd_no_adjustment_for_kc_below_045() {
        let crop_coefficients = CropCoefficientsGdd::new(
            "TestCrop".to_string(),
            (100.0, 0.3),  // Initial stage
            (200.0, 0.5),  // Development stage
            (300.0, 0.6),  // Mid-season stage
            (400.0, 0.2),  // Late stage
        );

        let cumulative_gdd = 350.0; // Beyond mid-season, in late stage
        let wind_speed = Some(3.0);
        let rh_min = Some(40.0);
        let crop_height = Some(1.0);

        let (crop_name, kc) = crop_coefficient_gdd(cumulative_gdd, crop_coefficients, wind_speed, rh_min, crop_height);

        assert_eq!(crop_name, "TestCrop");
        assert!((kc - 0.4).abs() < 0.001, "Kc should not be adjusted and remain 0.4");
    }

    #[test]
    // Should convert relative humidity to percentage if provided as a decimal less than 1
    fn test_crop_coefficient_gdd_with_rh_as_decimal() {
        let cc = CropCoefficientsGdd::new(
            "Wheat".to_string(),
            (200.0, 0.3),
            (500.0, 0.7),
            (800.0, 1.1),
            (1000.0, 0.5),
        );

        let cumulative_gdd = 600.0;
        let wind_speed = Some(3.0);
        let rh_min = Some(0.45); // Relative humidity as a decimal
        let crop_height = Some(1.0);

        let (crop_name, kc) = crop_coefficient_gdd(cumulative_gdd, cc, wind_speed, rh_min, crop_height);

        assert_eq!(crop_name, "Wheat");
        assert!((kc - 0.862).abs() < 0.01); // Expected Kc value after adjustment
    }

    #[test]
    // Should handle negative cumulative GDD by returning the initial stage Kc
    fn test_crop_coefficient_gdd_negative_cumulative_gdd() {
        let crop_name = "Wheat".to_string();
        let initial_end_kc = (100.0, 0.3);
        let development_end_kc = (200.0, 0.5);
        let mid_end_kc = (300.0, 1.2);
        let late_end_kc = (400.0, 0.8);

        let crop_coefficients = CropCoefficientsGdd::new(
            crop_name.clone(),
            initial_end_kc,
            development_end_kc,
            mid_end_kc,
            late_end_kc,
        );

        let cumulative_gdd = -50.0; // Negative GDD
        let (name, kc) = crop_coefficient_gdd(cumulative_gdd, crop_coefficients, None, None, None);

        assert_eq!(name, crop_name);
        assert!((kc - 0.3).abs() < 0.01, "Expected Kc to be 0.3, got {}", kc);
    }
}