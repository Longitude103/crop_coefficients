use crate::kc_gdd::adjust_kc;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

// Crop Coefficients GS struct to hold the mean coefficients for each crop stage using growth stage days, it contains the length of the
// period in days and the end Kc for each stage. Should use the FAO-56 crop coefficients.
#[derive(Debug)]
pub struct CropCoefficientsGs {
    pub crop_name: String,
    pub initial_end_kc: KcStage,
    pub development_end_kc: KcStage,
    pub mid_end_kc: KcStage,
    pub late_end_kc: KcStage,
    pub planting_date: NaiveDate,
    pub crop_height: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct KcStage {
    pub days: u16,
    pub kc: f32,
}

impl KcStage {
    pub fn new(days: u16, kc: f32) -> Self {
        KcStage { days, kc }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GrowthStage {
    Initial,
    Development,
    Mid,
    Late,
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
    planting_date: NaiveDate,
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
        planting_date: NaiveDate,
        crop_height: f64,
    ) -> CropCoefficientsGs {
        // check if Kc cannot exceed 2
        if initial_end_kc.1 > 2.0
            || development_end_kc.1 > 2.0
            || mid_end_kc.1 > 2.0
            || late_end_kc.1 > 2.0
        {
            panic!("Kc cannot exceed 2.");
        }

        let initial_stage = KcStage::new(initial_end_kc.0, initial_end_kc.1);
        let development_stage = KcStage::new(development_end_kc.0, development_end_kc.1);
        let mid_stage = KcStage::new(mid_end_kc.0, mid_end_kc.1);
        let late_stage = KcStage::new(late_end_kc.0, late_end_kc.1);

        CropCoefficientsGs {
            crop_name,
            initial_end_kc: initial_stage,
            development_end_kc: development_stage,
            mid_end_kc: mid_stage,
            late_end_kc: late_stage,
            planting_date,
            crop_height,
        }
    }

    /**
    Calculates the crop coefficient (Kc) with linear interpolation for Development and Late stages,
    and optional adjustments for environmental factors in Mid and Late stages.

    For Initial and Mid stages, Kc is constant. For Development and Late, it interpolates between
    the start and end Kc of the stage based on days into the stage.

    If wind_speed, rh_min, or crop_height are provided, adjusts Kc for Mid and Late stages using adjust_kc.

    # Parameters

    - `date`: A `NaiveDate` for which to calculate Kc.
    - `wind_speed`: Optional wind speed in m/s (default: 2.0).
    - `rh_min`: Optional minimum relative humidity in % (default: 45.0).
    - `crop_height`: Optional crop height in meters (default: 0.4).

    # Returns

    The calculated Kc value as f32.
    */
    pub fn coefficient_from_date(
        &self,
        date: NaiveDate,
        wind_speed: Option<f32>,
        rh_min: Option<f32>,
        crop_height: Option<f32>,
    ) -> f32 {
        let days_since_planting = date.signed_duration_since(self.planting_date).num_days() as i64;
        let growth_stage = self.determine_growth_stage(days_since_planting);

        let mut kc = match growth_stage {
            GrowthStage::Initial => self.initial_end_kc.kc,
            GrowthStage::Development => {
                let days_into = days_since_planting - (self.initial_end_kc.days as i64);
                let length = (self.development_end_kc.days - self.initial_end_kc.days) as i64;
                if length == 0 {
                    self.development_end_kc.kc
                } else {
                    self.initial_end_kc.kc
                        + (self.development_end_kc.kc - self.initial_end_kc.kc)
                            * (days_into as f32 / length as f32)
                }
            }
            GrowthStage::Mid => self.mid_end_kc.kc,
            GrowthStage::Late => {
                let days_into = days_since_planting - (self.mid_end_kc.days as i64);
                let length = (self.late_end_kc.days - self.mid_end_kc.days) as i64;
                if length == 0 {
                    self.late_end_kc.kc
                } else {
                    self.mid_end_kc.kc
                        + (self.late_end_kc.kc - self.mid_end_kc.kc)
                            * (days_into as f32 / length as f32)
                }
            }
        };

        if matches!(growth_stage, GrowthStage::Mid | GrowthStage::Late) {
            let wind_speed = wind_speed.unwrap_or(2.0);
            let rh_min = rh_min.unwrap_or(45.0);
            let crop_height = crop_height.unwrap_or(0.4);
            kc = adjust_kc(kc, wind_speed, rh_min, crop_height);
        }

        kc
    }

    fn determine_growth_stage(&self, days_since_planting: i64) -> GrowthStage {
        // determine which growth stage the crop is in based on the days since planting
        if days_since_planting <= self.initial_end_kc.days as i64 {
            GrowthStage::Initial
        } else if days_since_planting <= self.development_end_kc.days as i64 {
            GrowthStage::Development
        } else if days_since_planting <= self.mid_end_kc.days as i64 {
            GrowthStage::Mid
        } else {
            GrowthStage::Late
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

    if days_since_planting <= cc.initial_end_kc.days {
        (cc.crop_name, (cc.initial_end_kc.kc * 100.0).round() / 100.0) // Kc for initial stage
    } else if days_since_planting <= cc.development_end_kc.days {
        // Interpolation between initial and development stages
        (
            cc.crop_name,
            ((cc.initial_end_kc.kc
                + (cc.development_end_kc.kc - cc.initial_end_kc.kc)
                    * ((days_since_planting - cc.initial_end_kc.days)
                        / (cc.development_end_kc.days - cc.initial_end_kc.days))
                        as f32)
                * 100.0)
                / 100.0,
        ) // Kc for development stage
    } else if days_since_planting <= cc.mid_end_kc.days {
        // Interpolation between development and mid-season stages
        let kc_org = cc.development_end_kc.kc
            + (cc.mid_end_kc.kc - cc.development_end_kc.kc)
                * ((days_since_planting - cc.development_end_kc.days)
                    / (cc.mid_end_kc.days - cc.development_end_kc.days)) as f32;
        // Adjust Kc based on crop height and wind speed to compensate for arid and windy conditions
        (
            cc.crop_name,
            (adjust_kc(kc_org, wind_speed, rh_min, crop_height) * 100.0) / 100.0,
        ) // Kc for mid-season stage
    } else {
        // Interpolation between mid-season and end stages
        let kc_org = cc.mid_end_kc.kc
            - (cc.mid_end_kc.kc - cc.late_end_kc.kc)
                * ((days_since_planting - cc.late_end_kc.days)
                    / (cc.mid_end_kc.days - cc.late_end_kc.days)) as f32;
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

pub fn load_crop_coefficients(
) -> Result<HashMap<String, CropCoefficientsGs>, Box<dyn std::error::Error>> {
    // Read and parse the TOML file
    let toml_str = fs::read_to_string("fao56.toml")?;
    let crop_data: CropKcData = toml::from_str(&toml_str)?;

    // Convert the HashMap of crops into a HashMap<String, CropCoefficientsGs>
    let result: HashMap<String, CropCoefficientsGs> = crop_data
        .crops
        .into_iter()
        .map(|(_key, crop)| {
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
            let cc = CropCoefficientsGs::new(
                crop.name.clone(),
                (initial_days, crop.k_ini as f32),
                (development_days, crop.k_mid as f32), // Using k_mid as end of development
                (mid_days, crop.k_mid as f32),
                (late_days, crop.k_end as f32),
                crop.planting_date,
                crop.height_m,
            );
            (crop.name, cc)
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
        let corn_coefficient = crop_coefficients.get("corn").expect("Corn not found");
        assert_eq!(corn_coefficient.initial_end_kc.days, 20);
        assert_eq!(corn_coefficient.initial_end_kc.kc, 0.30);
        assert_eq!(corn_coefficient.development_end_kc.days, 50);
        assert_eq!(corn_coefficient.development_end_kc.kc, 1.20);
        assert_eq!(corn_coefficient.mid_end_kc.days, 100);
        assert_eq!(corn_coefficient.mid_end_kc.kc, 1.20);
        assert_eq!(corn_coefficient.late_end_kc.days, 120);
        assert_eq!(corn_coefficient.late_end_kc.kc, 0.60);
    }
}
