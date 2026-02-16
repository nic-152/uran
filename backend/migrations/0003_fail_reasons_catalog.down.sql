BEGIN;

DELETE FROM fail_reasons
WHERE code IN (
  'rtsp_stream_codec_bitrate',
  'onvif_compatibility',
  'network_dhcp_ipv6_multicast_ports',
  'video_wdr_night_noise_blur',
  'events_detection_false_positives',
  'recording_sd_nfs_ftp',
  'ui_settings_persistence',
  'performance_overheat_reboot'
);

COMMIT;
