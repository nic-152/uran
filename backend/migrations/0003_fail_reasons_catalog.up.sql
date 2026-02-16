BEGIN;

INSERT INTO fail_reasons (code, title, description, is_active)
VALUES
  ('rtsp_stream_codec_bitrate', 'RTSP/Stream/Codec/Bitrate', 'Проблемы с RTSP-потоками, кодеками, битрейтом, профилями потока.', TRUE),
  ('onvif_compatibility', 'ONVIF Compatibility', 'Проблемы ONVIF-совместимости, discovery, capabilities, профилей и клиентов.', TRUE),
  ('network_dhcp_ipv6_multicast_ports', 'Network (DHCP/IPv6/Multicast/Ports)', 'Сетевые проблемы: DHCP, IPv6, multicast/IGMP, VLAN, firewall, порты.', TRUE),
  ('video_wdr_night_noise_blur', 'Video (WDR/Night/Noise/Blur)', 'Дефекты изображения: WDR, ночной режим, шум, смаз, экспозиция, резкость.', TRUE),
  ('events_detection_false_positives', 'Events/Detection/False Positives', 'Проблемы детекции событий, ложные срабатывания, пропуски срабатываний.', TRUE),
  ('recording_sd_nfs_ftp', 'Recording (SD/NFS/FTP)', 'Проблемы записи и хранения: SD, NFS, FTP, ротация, целостность файлов.', TRUE),
  ('ui_settings_persistence', 'UI/Settings Persistence', 'Проблемы интерфейса и сохранения/применения настроек.', TRUE),
  ('performance_overheat_reboot', 'Performance/Overheat/Reboot', 'Проблемы производительности, перегрев, memory leak, неожиданные перезагрузки.', TRUE)
ON CONFLICT (code)
DO UPDATE SET
  title = EXCLUDED.title,
  description = EXCLUDED.description,
  is_active = EXCLUDED.is_active,
  updated_at = NOW();

COMMIT;
