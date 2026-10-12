use signals_sdr::export::formats::{export_iq_file, read_iq_file, IqExportFormat};
use std::fs;

#[test]
fn test_export_and_read_wav_iq() {
    let temp_dir = std::env::temp_dir().join("signals_sdr_test_wav_iq");
    let _ = fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("test_signal.wav");

    let original_samples: Vec<(f32, f32)> = (0..1000)
        .map(|i| {
            let t = i as f32 / 1000.0;
            ((t * 6.28).cos() * 0.8, (t * 6.28).sin() * 0.8)
        })
        .collect();

    let export_res = export_iq_file(
        &file_path,
        &original_samples,
        288_000,
        IqExportFormat::WavIq16,
    );
    assert!(export_res.is_ok(), "Export to WavIq16 failed");

    let read_res = read_iq_file(&file_path, IqExportFormat::WavIq16);
    assert!(read_res.is_ok(), "Reading back WavIq16 failed");
    let (read_samples, read_sr) = read_res.unwrap();

    assert_eq!(read_sr, 288_000);
    assert_eq!(read_samples.len(), original_samples.len());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_export_and_read_bladerf_sc16_q11() {
    let temp_dir = std::env::temp_dir().join("signals_sdr_test_bladerf_sc16");
    let _ = fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("test_signal.sc16");

    let original_samples: Vec<(f32, f32)> = vec![(0.5, -0.5), (0.0, 1.0), (-1.0, 0.0)];

    let export_res = export_iq_file(
        &file_path,
        &original_samples,
        2_000_000,
        IqExportFormat::RawSc16Q11,
    );
    assert!(export_res.is_ok(), "Export to RawSc16Q11 failed");

    let read_res = read_iq_file(&file_path, IqExportFormat::RawSc16Q11);
    assert!(read_res.is_ok(), "Reading back RawSc16Q11 failed");
    let (read_samples, _) = read_res.unwrap();

    assert_eq!(read_samples.len(), original_samples.len());
    // SC16_Q11 scaling verification
    assert!((read_samples[0].0 - 0.5).abs() < 0.01);
    assert!((read_samples[0].1 - (-0.5)).abs() < 0.01);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_export_and_read_hackrf_int8() {
    let temp_dir = std::env::temp_dir().join("signals_sdr_test_hackrf_int8");
    let _ = fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("test_signal.raw");

    let original_samples: Vec<(f32, f32)> = vec![(0.7, -0.7), (0.2, -0.4)];

    let export_res = export_iq_file(
        &file_path,
        &original_samples,
        2_000_000,
        IqExportFormat::RawInt8,
    );
    assert!(export_res.is_ok(), "Export to RawInt8 failed");

    let read_res = read_iq_file(&file_path, IqExportFormat::RawInt8);
    assert!(read_res.is_ok(), "Reading back RawInt8 failed");
    let (read_samples, _) = read_res.unwrap();

    assert_eq!(read_samples.len(), original_samples.len());

    let _ = fs::remove_dir_all(&temp_dir);
}
