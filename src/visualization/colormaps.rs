//! Color mapping functions for data visualization
//!
//! This module provides colormap implementations for mapping scalar values
//! to colors, particularly useful for heatmaps and data visualization.

/// Turbo colormap implementation
/// 
/// The Turbo colormap is a perceptually uniform colormap developed by Google
/// that provides excellent visualization for scientific data. It maps values
/// from 0.0 to 1.0 to colors ranging from dark blue through cyan, green, 
/// yellow, orange, to red.
///
/// # Arguments
/// * `t` - Normalized value between 0.0 and 1.0
///
/// # Returns
/// * RGBA color as [f32; 4] with values between 0.0 and 1.0
pub fn turbo_colormap(t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    
    // Turbo colormap polynomial coefficients for RGB channels
    // These are optimized coefficients that approximate the Turbo colormap
    // Source: https://gist.github.com/mikhailov-work/ee72ba4191942acecc03fe6da94fc73f
    
    let r = polynomial_turbo_r(t);
    let g = polynomial_turbo_g(t);
    let b = polynomial_turbo_b(t);
    
    [r, g, b, 1.0]
}

/// Red channel polynomial for Turbo colormap
fn polynomial_turbo_r(t: f32) -> f32 {
    let coeffs = [
        0.13572138,
        4.61539260,
        -42.66032258,
        132.13108234,
        -152.94239396,
        59.28637943,
    ];
    
    polynomial_eval(t, &coeffs).clamp(0.0, 1.0)
}

/// Green channel polynomial for Turbo colormap  
fn polynomial_turbo_g(t: f32) -> f32 {
    let coeffs = [
        0.09140261,
        2.19418839,
        4.84296658,
        -14.18503333,
        4.27729857,
        2.82956604,
    ];
    
    polynomial_eval(t, &coeffs).clamp(0.0, 1.0)
}

/// Blue channel polynomial for Turbo colormap
fn polynomial_turbo_b(t: f32) -> f32 {
    let coeffs = [
        0.10342779,
        -3.29743107,
        24.81307239,
        -78.43245046,
        93.38840218,
        -36.22902374,
    ];
    
    polynomial_eval(t, &coeffs).clamp(0.0, 1.0)
}

/// Evaluate polynomial with given coefficients
/// Coefficients are ordered from constant to highest degree
fn polynomial_eval(x: f32, coeffs: &[f32]) -> f32 {
    let mut result = 0.0;
    let mut x_power = 1.0;
    
    for &coeff in coeffs {
        result += coeff * x_power;
        x_power *= x;
    }
    
    result
}

/// Alternative simpler turbo colormap using piecewise linear approximation
#[allow(dead_code)]
pub fn turbo_colormap_simple(t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    
    let (r, g, b) = if t < 0.25 {
        // Dark blue to cyan (0.0 to 0.25)
        let s = t / 0.25;
        (0.0, s * 0.8, 1.0)
    } else if t < 0.5 {
        // Cyan to green (0.25 to 0.5)
        let s = (t - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - s)
    } else if t < 0.75 {
        // Green to yellow (0.5 to 0.75)
        let s = (t - 0.5) / 0.25;
        (s, 1.0, 0.0)
    } else {
        // Yellow to red (0.75 to 1.0)
        let s = (t - 0.75) / 0.25;
        (1.0, 1.0 - s, 0.0)
    };
    
    [r, g, b, 1.0]
}

/// Map array of counts to normalized colors with specified range mode
#[allow(dead_code)]
pub fn map_counts_to_colors(
    counts: &[u32],
    range_mode: crate::visualization::heatmap::RangeMode,
    fixed_max: Option<u32>,
    alpha: f32,
) -> Vec<[f32; 4]> {
    if counts.is_empty() {
        return Vec::new();
    }
    
    // Determine normalization range
    let (min_count, max_count) = match range_mode {
        crate::visualization::heatmap::RangeMode::Auto => {
            let min = *counts.iter().min().unwrap_or(&0);
            let max = *counts.iter().max().unwrap_or(&1);
            (min, max.max(1))
        },
        crate::visualization::heatmap::RangeMode::Fixed => {
            (0, fixed_max.unwrap_or(20))
        }
    };
    
    // Map each count to color
    counts.iter()
        .map(|&count| {
            let normalized = if max_count > min_count {
                (count - min_count) as f32 / (max_count - min_count) as f32
            } else {
                0.0
            };
            
            let mut color = turbo_colormap(normalized.clamp(0.0, 1.0));
            color[3] = alpha;
            color
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turbo_colormap_bounds() {
        // Test edge cases
        let color_min = turbo_colormap(0.0);
        let color_max = turbo_colormap(1.0);
        let color_mid = turbo_colormap(0.5);
        
        // All values should be in [0, 1] range
        for color in [color_min, color_max, color_mid] {
            for channel in color.iter() {
                assert!(*channel >= 0.0 && *channel <= 1.0, "Color channel out of range: {}", channel);
            }
        }
        
        // Alpha should always be 1.0 for turbo_colormap
        assert_eq!(color_min[3], 1.0);
        assert_eq!(color_max[3], 1.0);
        assert_eq!(color_mid[3], 1.0);
    }
    
    #[test] 
    fn test_turbo_colormap_progression() {
        // Test that colors progress smoothly
        let colors: Vec<_> = (0..=10)
            .map(|i| turbo_colormap(i as f32 / 10.0))
            .collect();
            
        // Should have distinct colors
        for window in colors.windows(2) {
            let diff = (window[0][0] - window[1][0]).abs() +
                      (window[0][1] - window[1][1]).abs() +
                      (window[0][2] - window[1][2]).abs();
            // Colors should be different (but this test is quite permissive)
            assert!(diff > 0.01, "Adjacent colors too similar");
        }
    }

    #[test]
    fn test_map_counts_to_colors() {
        let counts = vec![0, 5, 10, 15, 20];
        let colors = map_counts_to_colors(
            &counts, 
            crate::visualization::heatmap::RangeMode::Auto, 
            None, 
            0.8
        );
        
        assert_eq!(colors.len(), counts.len());
        
        // Check alpha channel
        for color in colors {
            assert_eq!(color[3], 0.8);
        }
    }
}