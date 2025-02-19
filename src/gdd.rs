/// Calculates the Growing Degree Days (GDD) based on daily temperature extremes and a base temperature.
///
/// This function computes the GDD, which is a measure of heat accumulation used to predict plant and insect development rates.
/// The GDD is calculated by taking the average of the daily maximum and minimum temperatures and subtracting the base temperature.
/// Ensure that the units of temperature are consistent (e.g., degrees Celsius or degrees Fahrenheit).
///
/// # Parameters
///
/// - `max_temp`: The maximum temperature for the day. It is constrained to be no less than 0 and no more than 30.
/// - `min_temp`: The minimum temperature for the day. It is constrained to be no less than -5 and no more than 30.
/// - `base_temp`: The base temperature, below which plant growth is assumed to be negligible. It is constrained to be no less than 0.
///
/// # Returns
///
/// Returns the GDD value as a `f32`. If the average temperature is less than or equal to the base temperature, the function returns 0.0.
/// Otherwise, it returns the difference between the average temperature and the base temperature.
pub fn calculate_gdd(mut max_temp: f32, mut min_temp: f32, mut base_temp: f32) -> f32 {
    max_temp = max_temp.max(0.0); // Limit max_temp to no less than 0 degrees
    max_temp = max_temp.min(30.0); // Limit max_temp to 30 degrees

    min_temp = min_temp.max(-5.0); // Limit min_temp to no less than -5.0 degrees
    min_temp = min_temp.min(30.0); // Limit min_temp to 30.0 degrees

    // Ensure the base_temp is not below 0
    base_temp = base_temp.max(0.0);

    // Ensure max_temp is not below min_temp
    let max_temp = max_temp.max(min_temp);

    // Calculate average temperature for the day
    let avg_temp = (max_temp + min_temp) / 2.0;

    // If avg_temp is less than or equal to base_temp, GDD is 0
    if avg_temp <= base_temp {
        0.0
    } else {
        // Calculate GDD by subtracting base_temp from avg_temp
        avg_temp - base_temp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_gdd_avg_temp_equals_base_temp() {
        let max_temp = 15.0;
        let min_temp = 5.0;
        let base_temp = 10.0;
        let result = calculate_gdd(max_temp, min_temp, base_temp);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_gdd_avg_temp_less_than_base_temp() {
        let max_temp = 12.0;
        let min_temp = 6.0;
        let base_temp = 10.0;
        let result = calculate_gdd(max_temp, min_temp, base_temp);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_gdd_avg_temp_above_base_temp() {
        let max_temp = 25.0;
        let min_temp = 15.0;
        let base_temp = 10.0;
        let result = calculate_gdd(max_temp, min_temp, base_temp);
        assert_eq!(result, 10.0);
    }

    #[test]
    fn test_calculate_gdd_with_negative_temperatures() {
        let max_temp = -5.0;
        let min_temp = -15.0;
        let base_temp = 0.0;
        let result = calculate_gdd(max_temp, min_temp, base_temp);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_gdd_with_very_large_temperatures() {
        let max_temp = 1_000_000.0;
        let min_temp = 999_999.0;
        let base_temp = 10.0;
        let result = calculate_gdd(max_temp, min_temp, base_temp);
        assert_eq!(result, 20.0);
    }

    #[test]
    fn test_calculate_gdd_adjust_max_temp() {
        let max_temp = 10.0;
        let min_temp = 15.0;
        let base_temp = 5.0;
        let result = calculate_gdd(max_temp, min_temp, base_temp);
        assert_eq!(result, 10.0);
    }
}

