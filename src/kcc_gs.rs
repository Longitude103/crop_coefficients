use crate::kc_gdd::adjust_kc;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

// Crop Coefficients GS struct to hold the mean coefficients for each crop stage using growth stage days, it contains the length of the
// period in days and the end Kc for each stage. Should use the FAO-56 crop coefficients.
#[derive(Debug)]
pub struct CropCoefficientsGs {
    crop_name: String,
    initial_end_kc: (u16, f32),
    development_end_kc: (u16, f32),
    mid_end_kc: (u16, f32),
    late_end_kc: (u16, f32),
}

// Define the Crop struct for individual crop data
#[derive(Debug, Serialize, Deserialize)]
struct Crop {
    name: String,
    k_ini: f64,                   // Initial stage coefficient
    k_mid: f64,                   // Mid-season coefficient
    k_end: f64,                   // Late-season coefficient
    height_m: f64,                // Crop height in meters
    growth_stages_days: Vec<i32>, // Growth stages in days [initial, dev, mid, late]
}

// Define the Climate struct for climate data
#[derive(Debug, Serialize, Deserialize)]
struct Climate {
    u2: f64,     // Wind speed at 2m height (m/s)
    rh_min: f64, // Minimum relative humidity (%)
}

// Define the root Config struct with a HashMap for crops
#[derive(Debug, Serialize, Deserialize)]
struct CropKcData {
    crops: HashMap<String, Crop>,
    climate: Climate,
}

impl CropCoefficientsGs {
    /// Creates a new instance of `CropCoefficientsGs` with specified parameters for each growth stage.
    ///
    /// # Parameters
    ///
    /// - `crop_name`: A `String` representing the name of the crop.
    /// - `initial_end_kc`: A tuple `(u16, f32)` representing the length of the initial growth period in days and the end mena Kc value for this stage.
    /// - `development_end_kc`: A tuple `(u16, f32)` representing the length of the development growth period in days and the end mean Kc value for this stage.
    /// - `mid_end_kc`: A tuple `(u16, f32)` representing the length of the mid-season growth period in days and the end mean Kc value for this stage.
    /// - `late_end_kc`: A tuple `(u16, f32)` representing the length of the late growth period in days and the end mean Kc value for this stage.
    ///
    /// # Returns
    ///
    /// A `CropCoefficients` struct initialized with the provided parameters. Panics if any Kc value exceeds 2.
    pub fn new(
        crop_name: String,
        initial_end_kc: (u16, f32),
        development_end_kc: (u16, f32),
        mid_end_kc: (u16, f32),
        late_end_kc: (u16, f32),
    ) -> CropCoefficientsGs {
        // check if Kc cannot exceed 2
        if initial_end_kc.1 > 2.0
            || development_end_kc.1 > 2.0
            || mid_end_kc.1 > 2.0
            || late_end_kc.1 > 2.0
        {
            panic!("Kc cannot exceed 2.");
        }

        CropCoefficientsGs {
            crop_name,
            initial_end_kc,
            development_end_kc,
            mid_end_kc,
            late_end_kc,
        }
    }
}

/// Calculates the crop coefficient (Kc) based on the length of each growth stage in days and other optional environmental factors.
/// It will adjust the Kc for wind speed, relative humidity, and crop height if provided the optional environmental factors.
///
/// # Parameters
///
/// - `planting_date`: A `NaiveDate` representing the planting date, which is used to determine the crop growth stage.
/// - `date`: A `NaiveDate` representing the current date requested for the crop coefficient.
/// - `cc`: A `CropCoefficients` struct containing the crop coefficients for different growth stages.
/// - `wind_speed`: An `Option<f32>` representing the wind speed in m/s. If not provided, defaults to 2.0 m/s.
/// - `rh_min`: An `Option<f32>` representing the minimum relative humidity in percentage. If not provided, defaults to 45.0%.
/// - `crop_height`: An `Option<f32>` representing the crop height in meters. If not provided, defaults to 0.0 m.
///
/// # Returns
///
/// A `(String, f32)` representing the name of the crop and the calculated crop coefficient (Kc) adjusted if given environmental conditions.
pub fn crop_coefficient_gs(
    planting_date: NaiveDate,
    date: NaiveDate,
    cc: CropCoefficientsGs,
    wind_speed: Option<f32>,
    rh_min: Option<f32>,
    crop_height: Option<f32>,
) -> (String, f32) {
    let wind_speed = wind_speed.unwrap_or(2.0);
    let mut rh_min = rh_min.unwrap_or(45.0);
    let crop_height = crop_height.unwrap_or(1.391);

    let days_since_planting = date.signed_duration_since(planting_date).num_days() as u16;

    if rh_min < 1.0 {
        rh_min *= 100.0; // Convert to percentage
    }

    if days_since_planting <= cc.initial_end_kc.0 {
        (cc.crop_name, (cc.initial_end_kc.1 * 100.0).round() / 100.0) // Kc for initial stage
    } else if days_since_planting <= cc.development_end_kc.0 {
        // Interpolation between initial and development stages
        (
            cc.crop_name,
            ((cc.initial_end_kc.1
                + (cc.development_end_kc.1 - cc.initial_end_kc.1)
                    * ((days_since_planting - cc.initial_end_kc.0)
                        / (cc.development_end_kc.0 - cc.initial_end_kc.0))
                        as f32)
                * 100.0)
                / 100.0,
        ) // Kc for development stage
    } else if days_since_planting <= cc.mid_end_kc.0 {
        // Interpolation between development and mid-season stages
        let kc_org = cc.development_end_kc.1
            + (cc.mid_end_kc.1 - cc.development_end_kc.1)
                * ((days_since_planting - cc.development_end_kc.0)
                    / (cc.mid_end_kc.0 - cc.development_end_kc.0)) as f32;
        // Adjust Kc based on crop height and wind speed to compensate for arid and windy conditions
        (
            cc.crop_name,
            (adjust_kc(kc_org, wind_speed, rh_min, crop_height) * 100.0) / 100.0,
        ) // Kc for mid-season stage
    } else {
        // Interpolation between mid-season and end stages
        let kc_org = cc.mid_end_kc.1
            - (cc.mid_end_kc.1 - cc.late_end_kc.1)
                * ((days_since_planting - cc.late_end_kc.0) / (cc.mid_end_kc.0 - cc.late_end_kc.0))
                    as f32;
        // let kc_org = (cc.development_end_kc.1 - ((cumulative_gdd - cc.mid_end_kc.0) / cc.late_end_kc.0)).max(cc.late_end_kc.1);
        // Adjust Kc based on crop height and wind speed if it's larger than 0.45
        if kc_org > 0.45 {
            (
                cc.crop_name,
                (adjust_kc(kc_org, wind_speed, rh_min, crop_height) * 100.0) / 100.0,
            ) // Kc for end stage
        } else {
            (cc.crop_name, (kc_org * 100.0) / 100.0) // Kc for end stage with default adjustment if Kc is less than 0.45
        }
    }
}

pub fn load_crop_coefficients() -> Result<Vec<CropCoefficientsGs>, Box<dyn std::error::Error>> {
    // Read and parse the TOML file
    let toml_str = fs::read_to_string("fao56.toml")?;
    let crop_data: CropKcData = toml::from_str(&toml_str)?;

    // Convert the HashMap of crops into a Vec of CropCoefficientsGs
    let result: Vec<CropCoefficientsGs> = crop_data
        .crops
        .into_values()
        .map(|crop| {
            // Calculate cumulative days for each stage end
            let initial_days = crop.growth_stages_days[0] as u16;
            let development_days = (crop.growth_stages_days[0] + crop.growth_stages_days[1]) as u16;
            let mid_days = (crop.growth_stages_days[0]
                + crop.growth_stages_days[1]
                + crop.growth_stages_days[2]) as u16;
            let late_days = (crop.growth_stages_days[0]
                + crop.growth_stages_days[1]
                + crop.growth_stages_days[2]
                + crop.growth_stages_days[3]) as u16;

            // Use the new method to create the struct
            CropCoefficientsGs::new(
                crop.name,
                (initial_days, crop.k_ini as f32),
                (development_days, crop.k_mid as f32), // Using k_mid as end of development
                (mid_days, crop.k_mid as f32),
                (late_days, crop.k_end as f32),
            )
        })
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_crop_coefficients() {
        let result = load_crop_coefficients();
        if result.is_err() {
            panic!("Error loading crop coefficients: {}", result.unwrap_err())
        }

        let crop_coefficients = result.unwrap();
        assert_eq!(crop_coefficients.len(), 12);

        // find a corn crop and check its coefficients
        let corn_coefficients = crop_coefficients.iter().find(|c| c.crop_name == "corn");
        assert!(corn_coefficients.is_some());
        let corn_coefficient = corn_coefficients.unwrap();
        assert_eq!(corn_coefficient.initial_end_kc.0, 20);
        assert_eq!(corn_coefficient.initial_end_kc.1, 0.30);
        assert_eq!(corn_coefficient.development_end_kc.0, 50);
        assert_eq!(corn_coefficient.development_end_kc.1, 1.20);
        assert_eq!(corn_coefficient.mid_end_kc.0, 100);
        assert_eq!(corn_coefficient.mid_end_kc.1, 1.20);
        assert_eq!(corn_coefficient.late_end_kc.0, 120);
        assert_eq!(corn_coefficient.late_end_kc.1, 0.60);
    }
}
