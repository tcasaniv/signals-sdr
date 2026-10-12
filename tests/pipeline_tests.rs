use fm_modulator::fm_modulate;
use am_modulator::{am_modulate_iq, AmMode};
use sstv_encoder::{encode_sstv, Mode};
use image::{DynamicImage, RgbImage};

#[test]
fn test_fm_modulator_pipeline_execution() {
    let audio_samples = vec![0.0f32, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5];
    let iq = fm_modulate(&audio_samples, 48_000, 288_000, 75_000.0, 0.0);
    assert!(!iq.is_empty());
    // Amplitud envolvente debe ser constante ~1.0
    for (i, q) in &iq {
        let mag = (i * i + q * q).sqrt();
        assert!((mag - 1.0).abs() < 0.01);
    }
}

#[test]
fn test_am_modulator_pipeline_execution() {
    let audio_samples = vec![0.0f32, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5];
    let iq_dsb = am_modulate_iq(&audio_samples, 48_000, 288_000, 0.85, AmMode::DsbFc, 0.0);
    assert!(!iq_dsb.is_empty());

    let iq_ssb = am_modulate_iq(&audio_samples, 48_000, 288_000, 1.0, AmMode::SsbUsb, 0.0);
    assert!(!iq_ssb.is_empty());
}

#[test]
fn test_sstv_pipeline_execution() {
    // Imagen mínima de prueba Robot 36 (320x240)
    let img = DynamicImage::ImageRgb8(RgbImage::new(320, 240));
    let samples = encode_sstv(&img, Mode::Robot36, 48_000);
    assert!(samples.is_ok());
    let audio = samples.unwrap();
    assert!(!audio.is_empty());
}
