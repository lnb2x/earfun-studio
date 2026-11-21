pub mod audio;
pub mod backend;
pub mod presets;
pub mod protocol;
pub mod settings;
pub mod ui;
pub mod workflows;

#[cfg(test)]
mod tests {
    use super::*;
    use backend::{BackendCore, BackendError, BoxFuture, GattClient};
    use protocol::{self as p, DeviceState, Reply};
    use settings::Settings;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, Weak};
    use std::time::Duration;

    #[test]
    fn test_reference_packets() {
        assert_eq!(p::hex_encode(&p::request(20, &[1])), "000a031401");
        assert_eq!(
            p::hex_encode(&p::gesture_packet("L", "Nhấn giữ", "Phát / tạm dừng").unwrap()),
            "000a030a11010000"
        );
        assert_eq!(
            p::hex_encode(&p::gesture_packet("R", "Chạm hai lần", "Bài tiếp").unwrap()),
            "000a030a00000205"
        );
        let eq = p::equalizer(&[0; 10]).unwrap();
        assert_eq!(p::hex_encode(&eq[0]), "001d0e01a1fe98005e00000b33");
        assert_eq!(p::hex_encode(&eq[9]), "001d0e01aafe98bb8000000b33");
        assert_eq!(eq.len(), 10);
        let eq_min = p::equalizer(&[-10; 10]).unwrap();
        assert_eq!(&eq_min[0][9..11], b"\xfe\x98");
        let eq_max = p::equalizer(&[10; 10]).unwrap();
        assert_eq!(&eq_max[0][9..11], b"\x01\x68");
    }

    #[test]
    fn test_invalid_packets_and_gains() {
        for data in [&b""[..], &b"\x00\x0a\x83"[..], &b"\x00\x0a\x03\x06\x00"[..]] {
            assert!(p::decode(data).is_none());
        }
        assert!(p::equalizer(&[0; 9]).is_err());
        assert!(p::equalizer(&[11; 10]).is_err());
        assert!(p::equalizer(&[-11; 10]).is_err());
        assert!(p::gesture_packet("X", "Chạm một lần", "Phát / tạm dừng").is_err());
    }

    #[test]
    fn test_decode_state_unknown_and_rejected() {
        let battery = p::decode(&p::hex_decode("000a83060057").unwrap()).unwrap();
        assert_eq!(p::state(&battery).bat_l, Some(87));

        for raw in [
            "000a830600ff",
            "000a83060157",
            "001d83060057",
            "000a831500ff",
        ] {
            let decoded = p::decode(&p::hex_decode(raw).unwrap()).unwrap();
            assert!(p::state(&decoded).is_empty());
        }
        let ear = p::decode(&p::hex_decode("000a834a0000").unwrap()).unwrap();
        assert_eq!(p::state(&ear).ear, Some(true));

        let codecs = p::decode(&p::hex_decode("000a832f001b").unwrap()).unwrap();
        let st = p::state(&codecs);
        assert_eq!(st.aptx, Some(true));
        assert_eq!(st.adaptive, Some(true));
        assert_eq!(st.ldac, Some(true));
        assert_eq!(st.lossless, Some(true));
    }

    #[test]
    fn test_streamed_phones_and_short_keys() {
        let mut raw = p::hex_decode("000a032c00").unwrap();
        raw.push(0);
        let mut name_bytes = b"Ubuntu".to_vec();
        name_bytes.resize(19, 0);
        raw.extend_from_slice(&name_bytes);
        raw.extend_from_slice(b"\0\0");
        raw.extend_from_slice(&p::hex_decode("aabbcc").unwrap());
        raw.push(1);

        let reply = p::decode(&raw).unwrap();
        let phone = p::state(&reply).phone.unwrap();
        assert_eq!(phone.name, "Ubuntu");
        assert_eq!(phone.mac, "AABBCC");
        assert!(phone.connected);

        assert!(p::state(&Reply {
            vendor: 10,
            command: 0x30b,
            status: 0,
            payload: vec![0],
        })
        .keys
        .is_empty());

        let short_keys = p::state(&Reply {
            vendor: 10,
            command: 0x30b,
            status: 0,
            payload: vec![1, 1, 2, 5],
        });
        assert_eq!(
            short_keys.keys.get("L.Chạm một lần").map(|s| s.as_str()),
            Some("Phát / tạm dừng")
        );
        assert_eq!(
            short_keys.keys.get("R.Chạm một lần").map(|s| s.as_str()),
            Some("Bài tiếp")
        );
    }

    #[test]
    fn test_apk_catalog_identity_and_exact_reference_curves() {
        let builtins = presets::builtins();
        assert_eq!(builtins.len(), 32);
        let curves: BTreeMap<String, Vec<i32>> =
            builtins.into_iter().map(|r| (r.key, r.levels)).collect();
        assert_eq!(curves.len(), 32);
        assert_eq!(
            curves.get("earfun:24").unwrap(),
            &vec![4, 9, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            curves.get("earfun:29").unwrap(),
            &vec![0, 0, 0, 1, 4, 5, 4, 2, 0, 0]
        );
        assert_eq!(
            curves.get("fernanda:201").unwrap(),
            &vec![-2, 2, 0, 1, 2, 3, -1, -3, -4, 0]
        );
        assert_eq!(
            curves.get("fernanda:202").unwrap(),
            &vec![-7, -3, -1, -2, 0, 2, 2, -1, -2, 1]
        );
        assert_eq!(presets::catalog().source.version, "20.1.61");

        let bass = p::equalizer(curves.get("earfun:24").unwrap()).unwrap();
        assert_eq!(&bass[1][9..11], &p::hex_decode("012c").unwrap()[..]);
        for levels in curves.values() {
            assert_eq!(p::equalizer(levels).unwrap().len(), 10);
        }
    }

    #[test]
    fn test_personal_names_cannot_shadow_official_values() {
        let mut custom = BTreeMap::new();
        custom.insert("Rock".to_string(), vec![9; 10]);
        let opts = presets::options(&custom);
        let official = opts.iter().find(|r| r.key == "earfun:5").unwrap();
        let personal = opts.iter().find(|r| r.key == "custom:Rock").unwrap();
        assert_ne!(official.label, personal.label);
        assert_eq!(
            official.levels.as_ref().unwrap(),
            &vec![4, 5, 2, 0, 0, 1, 2, 3, 3, 2]
        );
        assert_eq!(personal.levels.as_ref().unwrap(), &vec![9; 10]);
    }

    #[test]
    fn test_export_import_is_lossless_and_rejects_wrong_model() {
        let levels: Vec<i32> = (-5..5).collect();
        let doc = presets::encode("Tên tiếng Việt", &levels).unwrap();
        let val = serde_json::to_value(&doc).unwrap();
        assert_eq!(
            presets::decode_value(&val).unwrap(),
            ("Tên tiếng Việt".to_string(), levels)
        );

        let mut bad_model = val.clone();
        bad_model["model"] = serde_json::json!("OTHER");
        assert!(presets::decode_value(&bad_model).is_err());

        let mut bad_freqs = val.clone();
        bad_freqs["frequencies"] = serde_json::to_value(vec![100; 10]).unwrap();
        assert!(presets::decode_value(&bad_freqs).is_err());

        let mut bad_bool_levels = val.clone();
        bad_bool_levels["levels"] = serde_json::to_value(vec![true; 10]).unwrap();
        assert!(presets::decode_value(&bad_bool_levels).is_err());

        let mut bad_range_levels = val.clone();
        bad_range_levels["levels"] = serde_json::to_value(vec![11; 10]).unwrap();
        assert!(presets::decode_value(&bad_range_levels).is_err());

        let mut bad_len_levels = val.clone();
        bad_len_levels["levels"] = serde_json::to_value(vec![0; 9]).unwrap();
        assert!(presets::decode_value(&bad_len_levels).is_err());

        let mut bad_ver = val.clone();
        bad_ver["version"] = serde_json::json!(2);
        assert!(presets::decode_value(&bad_ver).is_err());

        let mut bad_name = val;
        bad_name["name"] = serde_json::json!("");
        assert!(presets::decode_value(&bad_name).is_err());
    }

    #[test]
    fn test_fast_pair_is_inverse_of_le_audio_and_unknown_language_stays_unknown() {
        let st0 = p::state(&Reply {
            vendor: 10,
            command: 0x34c,
            status: 0,
            payload: vec![0],
        });
        assert_eq!(st0.le_audio, Some(false));
        assert_eq!(st0.google_fast_pair, Some(true));

        let st1 = p::state(&Reply {
            vendor: 10,
            command: 0x34c,
            status: 0,
            payload: vec![1],
        });
        assert_eq!(st1.le_audio, Some(true));
        assert_eq!(st1.google_fast_pair, Some(false));

        let lang_en = p::state(&Reply {
            vendor: 10,
            command: 0x371,
            status: 0,
            payload: vec![0],
        });
        assert_eq!(lang_en.voice_language.as_deref(), Some("EN"));

        let lang_unk = p::state(&Reply {
            vendor: 10,
            command: 0x371,
            status: 0,
            payload: vec![0xff],
        });
        assert!(lang_unk.is_empty());
        assert!(!p::QUERIES.contains(&113));
    }

    #[test]
    fn test_settings_validation_and_atomic_save() {
        let dir = std::env::temp_dir().join(format!("earfun_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("settings.json");

        std::fs::write(&path, "{bad").unwrap();
        let mut settings = Settings::new(Some(&path));
        assert_eq!(settings.data.eq, vec![0; 10]);

        settings
            .data
            .presets
            .insert("Riêng".to_string(), vec![3; 10]);
        settings.save().unwrap();
        assert_eq!(
            Settings::new(Some(&path))
                .data
                .presets
                .get("Riêng")
                .unwrap(),
            &vec![3; 10]
        );

        std::fs::write(
            &path,
            serde_json::json!({
                "eq": vec![90; 10],
                "presets": {"bad": [1]},
                "auto_connect": "false",
                "connection_policy_version": 2
            })
            .to_string(),
        )
        .unwrap();
        let reloaded = Settings::new(Some(&path));
        assert_eq!(reloaded.data.eq, vec![0; 10]);
        assert!(reloaded.data.presets.is_empty());
        assert!(!reloaded.data.auto_connect);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn startup_migrates_old_defaults_but_remembers_new_opt_out() {
        let dir = std::env::temp_dir().join(format!("earfun_startup_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let last_mac = "AA:BB:CC:DD:EE:FF";
        std::fs::write(
            &path,
            serde_json::json!({
                "auto_connect": false, "last_mac": last_mac,
                "eq": vec![2; 10], "presets": {"My EQ": vec![3; 10]}
            })
            .to_string(),
        )
        .unwrap();
        let mut settings = Settings::new(Some(&path));
        assert!(settings.data.auto_connect);
        assert_eq!(settings.data.last_mac, last_mac);
        assert_eq!(settings.data.eq, vec![2; 10]);
        assert_eq!(settings.data.presets["My EQ"], vec![3; 10]);
        settings.data.auto_connect = false;
        settings.save().unwrap();
        assert!(!Settings::new(Some(&path)).data.auto_connect);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn automatic_selection_never_picks_another_remembered_headset() {
        use backend::select_auto_device;
        let mut devices = BTreeMap::new();
        assert_eq!(select_auto_device(&devices, ""), None);
        devices.insert("AA:BB:CC:DD:EE:FF".into(), "EarFun Air Pro 4".into());
        assert_eq!(
            select_auto_device(&devices, ""),
            Some("AA:BB:CC:DD:EE:FF".into())
        );
        assert_eq!(
            select_auto_device(&devices, "aa:bb:cc:dd:ee:ff"),
            Some("AA:BB:CC:DD:EE:FF".into())
        );
        assert_eq!(select_auto_device(&devices, "11:22:33:44:55:66"), None);
        devices.insert("11:22:33:44:55:66".into(), "EarFun Air Pro 4".into());
        assert_eq!(select_auto_device(&devices, ""), None);
        assert_eq!(
            select_auto_device(&devices, "AA:BB:CC:DD:EE:FF"),
            Some("AA:BB:CC:DD:EE:FF".into())
        );
    }

    #[tokio::test]
    async fn shutdown_prevents_automatic_discovery() {
        let core = BackendCore::new(Arc::new(|_| panic!("No discovery after shutdown")));
        core.closing.store(true, Ordering::SeqCst);
        core.auto_connect("").await.unwrap();
    }

    #[test]
    fn test_never_select_speakers_or_another_bluetooth_sink() {
        let make_node = |id: i64, name: &str, media: &str| -> serde_json::Value {
            serde_json::json!({
                "id": id,
                "info": {
                    "props": {
                        "node.name": name,
                        "media.class": media,
                        "object.serial": id + 100
                    }
                }
            })
        };
        let mut objects = serde_json::json!([
            make_node(1, "alsa_output.AA_BB_CC_DD_EE_FF", "Audio/Sink"),
            make_node(2, "bluez_output.11_22_33_44_55_66.1", "Audio/Sink"),
            make_node(3, "bluez_output.AA_BB_CC_DD_EE_FF.1", "Audio/Source")
        ]);
        assert_eq!(audio::find_sink(&objects, "AA:BB:CC:DD:EE:FF"), None);
        objects.as_array_mut().unwrap().push(make_node(
            4,
            "bluez_output.AA_BB_CC_DD_EE_FF.1",
            "Audio/Sink",
        ));
        assert_eq!(
            audio::find_sink(&objects, "AA:BB:CC:DD:EE:FF"),
            Some("4".to_string())
        );
        assert_eq!(audio::find_sink(&objects, ""), None);
    }

    #[test]
    fn test_planner_workflows() {
        let mut st = DeviceState {
            dual: Some(true),
            ldac: Some(false),
            aptx: Some(true),
            adaptive: Some(true),
            lossless: Some(true),
            ..Default::default()
        };

        let mut wanted = BTreeMap::new();
        for &k in workflows::CODECS {
            wanted.insert(k.to_string(), st.get_bool(k).unwrap());
        }
        wanted.insert("ldac".to_string(), true);

        let plan = workflows::codecs(&st, &wanted).unwrap();
        let setters: Vec<u8> = plan.steps.iter().map(|s| s.setter).collect();
        assert_eq!(setters, vec![39, 46]);
        assert!(plan.confirmation().contains("Tắt kết nối hai thiết bị"));

        st.dual = Some(false);
        st.ldac = Some(true);
        let dual_plan = workflows::dual(&st, true).unwrap();
        let pairs: Vec<(u8, u8)> = dual_plan
            .steps
            .iter()
            .map(|s| (s.setter, s.value))
            .collect();
        assert_eq!(pairs, vec![(46, 19), (39, 1)]);

        st.dual = Some(true);
        st.ldac = Some(false);
        assert!(workflows::dual(&st, true).unwrap().steps.is_empty());
        assert!(workflows::dual(&DeviceState::default(), true).is_err());

        let le_state = DeviceState {
            le_audio: Some(true),
            ..Default::default()
        };
        let le_plan = workflows::restart_setting(&le_state, "le_audio", false).unwrap();
        assert_eq!(
            (
                le_plan.steps[0].setter,
                le_plan.steps[0].query,
                le_plan.steps[0].value
            ),
            (75, 76, 0)
        );
    }

    struct FakeClient {
        core: Weak<BackendCore>,
        connected: AtomicBool,
        writes: Mutex<Vec<Vec<u8>>>,
        noise: Mutex<u8>,
        reject: AtomicBool,
        wrong_vendor_first: AtomicBool,
        silent: AtomicBool,
    }

    impl FakeClient {
        fn new(core: &Arc<BackendCore>) -> Arc<Self> {
            Arc::new(Self {
                core: Arc::downgrade(core),
                connected: AtomicBool::new(true),
                writes: Mutex::new(Vec::new()),
                noise: Mutex::new(0),
                reject: AtomicBool::new(false),
                wrong_vendor_first: AtomicBool::new(false),
                silent: AtomicBool::new(false),
            })
        }
    }

    impl GattClient for FakeClient {
        fn is_connected(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
            Box::pin(async move {
                self.writes.lock().unwrap().push(packet.clone());
                if self.silent.load(Ordering::SeqCst) {
                    return Ok(());
                }
                let vendor = u16::from_be_bytes([packet[0], packet[1]]);
                let command = u16::from_be_bytes([packet[2], packet[3]]);
                if command == 0x0314 {
                    *self.noise.lock().unwrap() = packet[4];
                }
                let payload = if command == 0x0315 {
                    vec![*self.noise.lock().unwrap()]
                } else {
                    vec![]
                };
                let mut raw = Vec::new();
                raw.extend_from_slice(&vendor.to_be_bytes());
                raw.extend_from_slice(&(command | 0x8000).to_be_bytes());
                raw.push(u8::from(self.reject.load(Ordering::SeqCst)));
                raw.extend_from_slice(&payload);

                if let Some(core) = self.core.upgrade() {
                    if self.wrong_vendor_first.load(Ordering::SeqCst) {
                        let mut wrong = vec![0x00, 0x1d];
                        wrong.extend_from_slice(&raw[2..]);
                        core.notify(&wrong);
                    }
                    core.notify(&raw);
                }
                Ok(())
            })
        }

        fn close_session(&self) -> BoxFuture<'_, ()> {
            Box::pin(async move {
                self.connected.store(false, Ordering::SeqCst);
            })
        }
    }

    fn setup_test_core() -> (Arc<BackendCore>, Arc<FakeClient>) {
        let core = BackendCore::new(Arc::new(|_| {}));
        *core.step_sleep.lock().unwrap() = Duration::from_millis(1);
        core.state.lock().unwrap().version = Some("TW600-test".to_string());
        let client = FakeClient::new(&core);
        *core.client.lock().unwrap() = Some(client.clone());
        (core, client)
    }

    #[tokio::test]
    async fn test_setter_reads_actual_device_state() {
        let (core, client) = setup_test_core();
        core.set_string_value(20, 21, 1, "noise", "anc")
            .await
            .unwrap();
        assert_eq!(core.state.lock().unwrap().noise.as_deref(), Some("anc"));
        let writes = client.writes.lock().unwrap().clone();
        assert_eq!(
            writes,
            vec![
                p::hex_decode("000a031401").unwrap(),
                p::hex_decode("000a0315").unwrap()
            ]
        );
    }

    #[tokio::test]
    async fn test_request_matches_vendor_and_reports_rejection() {
        let (core, client) = setup_test_core();
        client.wrong_vendor_first.store(true, Ordering::SeqCst);
        let resp = core.request(&p::request(21, &[])).await.unwrap();
        assert_eq!(resp.vendor, 10);

        client.reject.store(true, Ordering::SeqCst);
        let err = core.request(&p::request(21, &[])).await.unwrap_err();
        assert!(matches!(err, BackendError::CommandRejected(_)));
    }

    #[tokio::test]
    async fn test_disconnect_stops_finder_first() {
        let (core, client) = setup_test_core();
        core.finding.store(true, Ordering::SeqCst);
        core.disconnect().await;
        let writes = client.writes.lock().unwrap().clone();
        assert_eq!(
            writes.last().unwrap(),
            &p::hex_decode("000a033200").unwrap()
        );
        assert!(!client.is_connected());
        assert!(core.state.lock().unwrap().version.is_none());
    }

    #[tokio::test]
    async fn test_no_eq_write_when_disconnected() {
        let (core, client) = setup_test_core();
        client.connected.store(false, Ordering::SeqCst);
        assert!(core.set_eq(&[0; 10]).await.is_err());
        assert!(client.writes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_timeout_closes_connection_and_clears_pending() {
        let (core, client) = setup_test_core();
        client.silent.store(true, Ordering::SeqCst);
        *core.request_timeout.lock().unwrap() = Duration::from_millis(20);
        assert!(core.request(&p::request(21, &[])).await.is_err());
        assert!(!client.is_connected());
        assert!(core.state.lock().unwrap().version.is_none());
        assert!(core.pending.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_ldac_conflict_has_no_writes() {
        let (core, client) = setup_test_core();
        {
            let mut st = core.state.lock().unwrap();
            st.dual = Some(true);
            st.ldac = Some(false);
            st.aptx = Some(true);
            st.adaptive = Some(true);
            st.lossless = Some(true);
        }
        let mut wanted = BTreeMap::new();
        wanted.insert("ldac".to_string(), true);
        wanted.insert("aptx".to_string(), true);
        wanted.insert("adaptive".to_string(), true);
        wanted.insert("lossless".to_string(), true);
        assert!(core.codecs(&wanted).await.is_err());
        assert!(client.writes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_workflow_reboot_and_partial_failure() {
        struct SimDevice {
            state: DeviceState,
            writes: Vec<(u8, u8)>,
            fail_codec: bool,
        }

        struct SimSession {
            core: Weak<BackendCore>,
            device: Arc<Mutex<SimDevice>>,
            connected: AtomicBool,
        }

        impl GattClient for SimSession {
            fn is_connected(&self) -> bool {
                self.connected.load(Ordering::SeqCst)
            }

            fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
                Box::pin(async move {
                    let cmd = packet[3];
                    let reply_value = {
                        let mut dev = self.device.lock().unwrap();
                        if cmd == 39 || cmd == 46 {
                            let val = packet[4];
                            dev.writes.push((cmd, val));
                            if cmd == 39 {
                                dev.state.dual = Some(val != 0);
                                self.connected.store(false, Ordering::SeqCst);
                                Some(None)
                            } else {
                                if !dev.fail_codec {
                                    dev.state.aptx = Some((val & 1) != 0);
                                    dev.state.adaptive = Some((val & 2) != 0);
                                    dev.state.ldac = Some((val & 8) != 0);
                                    dev.state.lossless = Some((val & 16) != 0);
                                }
                                return Ok(());
                            }
                        } else {
                            let value = if cmd == 38 {
                                u8::from(dev.state.dual.unwrap())
                            } else if cmd == 47 {
                                let mut bits = 0u8;
                                if dev.state.aptx.unwrap() {
                                    bits |= 1;
                                }
                                if dev.state.adaptive.unwrap() {
                                    bits |= 2;
                                }
                                if dev.state.ldac.unwrap() {
                                    bits |= 8;
                                }
                                if dev.state.lossless.unwrap() {
                                    bits |= 16;
                                }
                                bits
                            } else {
                                panic!("Unexpected command {}", cmd);
                            };
                            Some(Some(value))
                        }
                    };
                    match reply_value {
                        Some(None) => {
                            if let Some(core) = self.core.upgrade() {
                                core.disconnect_internal().await;
                            }
                        }
                        Some(Some(value)) => {
                            if let Some(core) = self.core.upgrade() {
                                let raw =
                                    p::packet(10, 0x8300 | (cmd as u16), &[0, value]).unwrap();
                                core.notify(&raw);
                            }
                        }
                        None => {}
                    }
                    Ok(())
                })
            }

            fn close_session(&self) -> BoxFuture<'_, ()> {
                Box::pin(async move {
                    self.connected.store(false, Ordering::SeqCst);
                })
            }
        }

        let init_state = DeviceState {
            version: Some("TW600-test".to_string()),
            dual: Some(true),
            ldac: Some(false),
            aptx: Some(true),
            adaptive: Some(true),
            lossless: Some(true),
            ..Default::default()
        };

        let dev = Arc::new(Mutex::new(SimDevice {
            state: init_state.clone(),
            writes: Vec::new(),
            fail_codec: false,
        }));

        let core = BackendCore::new(Arc::new(|_| {}));
        *core.step_sleep.lock().unwrap() = Duration::from_millis(1);
        *core.address.lock().unwrap() = Some("AA:BB:CC:DD:EE:FF".to_string());
        *core.state.lock().unwrap() = init_state.clone();
        *core.client.lock().unwrap() = Some(Arc::new(SimSession {
            core: Arc::downgrade(&core),
            device: dev.clone(),
            connected: AtomicBool::new(true),
        }));

        let reconnections = Arc::new(Mutex::new(0usize));
        let reconn_counter = reconnections.clone();
        let weak_core = Arc::downgrade(&core);
        let dev_for_reconn = dev.clone();
        core.set_reconnect_hook(Arc::new(move |_addr, _full| {
            let reconn_counter = reconn_counter.clone();
            let weak_core = weak_core.clone();
            let dev_for_reconn = dev_for_reconn.clone();
            Box::pin(async move {
                *reconn_counter.lock().unwrap() += 1;
                if let Some(c) = weak_core.upgrade() {
                    let st = dev_for_reconn.lock().unwrap().state.clone();
                    *c.state.lock().unwrap() = st;
                    *c.client.lock().unwrap() = Some(Arc::new(SimSession {
                        core: Arc::downgrade(&c),
                        device: dev_for_reconn,
                        connected: AtomicBool::new(true),
                    }));
                }
                Ok(())
            })
        }));

        let weak_core2 = Arc::downgrade(&core);
        let dev_for_query = dev.clone();
        core.set_query_hook(Arc::new(move |_full| {
            let weak_core2 = weak_core2.clone();
            let dev_for_query = dev_for_query.clone();
            Box::pin(async move {
                if let Some(c) = weak_core2.upgrade() {
                    *c.state.lock().unwrap() = dev_for_query.lock().unwrap().state.clone();
                }
                Ok(())
            })
        }));

        let mut wanted = BTreeMap::new();
        for &k in workflows::CODECS {
            wanted.insert(k.to_string(), init_state.get_bool(k).unwrap());
        }
        wanted.insert("ldac".to_string(), true);
        let plan = workflows::codecs(&init_state, &wanted).unwrap();

        core.run_plan(&plan).await.unwrap();
        assert_eq!(dev.lock().unwrap().writes, vec![(39, 0), (46, 27)]);
        assert_eq!(*reconnections.lock().unwrap(), 1);
        assert_eq!(core.state.lock().unwrap().ldac, Some(true));
        assert_eq!(core.state.lock().unwrap().dual, Some(false));

        // External change during confirmation causes zero writes
        dev.lock().unwrap().writes.clear();
        dev.lock().unwrap().state = init_state.clone();
        *core.state.lock().unwrap() = init_state.clone();
        dev.lock().unwrap().state.aptx = Some(false);
        let err = core.run_plan(&plan).await.unwrap_err().to_string();
        assert!(err.contains("đã đổi"));
        assert!(dev.lock().unwrap().writes.is_empty());

        // Partial failure reports completed step and never resends
        dev.lock().unwrap().writes.clear();
        dev.lock().unwrap().state = init_state.clone();
        dev.lock().unwrap().fail_codec = true;
        *core.state.lock().unwrap() = init_state.clone();
        let err = core.run_plan(&plan).await.unwrap_err().to_string();
        assert!(err.contains("Đã hoàn tất: Tắt kết nối hai thiết bị"));
        assert_eq!(dev.lock().unwrap().writes, vec![(39, 0), (46, 27)]);

        // Closing stops retry without another write
        dev.lock().unwrap().writes.clear();
        core.closing.store(true, Ordering::SeqCst);
        assert!(core.run_plan(&plan).await.is_err());
        assert!(dev.lock().unwrap().writes.is_empty());
    }

    #[test]
    fn test_stage1_protocol_utf8_hex_version_gestures_and_redaction() {
        // UTF-8 multibyte input must never panic on char boundary slicing
        assert!(p::hex_decode("aéb").is_err());
        assert!(p::hex_decode("việt").is_err());
        assert_eq!(p::hex_decode("00 0a 83").unwrap(), vec![0x00, 0x0a, 0x83]);

        // Oversized payload must return Err rather than panicking
        assert!(p::try_request(20, &vec![0u8; 600]).is_err());

        // Fast Pair <-> LE Audio bidirectional invariant in set_bool
        let mut st = DeviceState::default();
        st.set_bool("google_fast_pair", true);
        assert_eq!(st.google_fast_pair, Some(true));
        assert_eq!(st.le_audio, Some(false));
        st.set_bool("google_fast_pair", false);
        assert_eq!(st.google_fast_pair, Some(false));
        assert_eq!(st.le_audio, Some(true));

        // Firmware version code extraction per APK TW600Config (x.y.z -> 100*x + 10*y + z)
        assert_eq!(
            p::parse_firmware_version_code("TW600-3091_20250919_v0.4.1"),
            Some(41)
        );
        assert_eq!(p::parse_firmware_version_code("TW600-0.2.7"), Some(27));
        assert_eq!(p::parse_firmware_version_code("TW600-1.3.5"), Some(135));
        assert_eq!(p::parse_firmware_version_code("TW600-test"), None);

        // Gesture operation filtering per TW600Config
        let single_ops = p::allowed_operations("Chạm một lần", Some("TW600-0.4.9"));
        assert!(single_ops.iter().any(|(_, c)| *c == 1));
        assert!(!single_ops.iter().any(|(_, c)| *c == 7)); // No Game Mode on single tap
        assert!(!single_ops.iter().any(|(_, c)| *c == 9)); // No custom ANC on single tap
        assert!(!single_ops.iter().any(|(_, c)| *c == 13)); // No Redial on single tap

        let triple_old_fw = p::allowed_operations("Chạm ba lần", Some("TW600-0.2.7"));
        assert!(triple_old_fw.iter().any(|(_, c)| *c == 7));
        assert!(triple_old_fw.iter().any(|(_, c)| *c == 9));
        assert!(!triple_old_fw.iter().any(|(_, c)| *c == 13)); // Redial requires > 27

        let triple_new_fw =
            p::allowed_operations("Chạm ba lần", Some("TW600-3091_20250919_v0.4.1"));
        assert!(triple_new_fw.iter().any(|(_, c)| *c == 13));

        // Privacy redaction masks MAC addresses while keeping BLE spaced hex intact
        let raw_log = "12:00:01 >> 00 0a 03 14 01 dev AA:BB:CC:DD:EE:FF node bluez_output.11_22_33_44_55_66.1";
        let redacted = p::redact_sensitive_identifiers(raw_log);
        assert!(redacted.contains("00 0a 03 14 01"));
        assert!(!redacted.contains("AA:BB:CC:DD:EE:FF"));
        assert!(!redacted.contains("11_22_33_44_55_66"));
        assert!(redacted.contains("AA:BB:**:**:EE:FF"));
    }

    #[test]
    fn test_stage1_settings_backup_bare_path_and_theme_persistence() {
        let dir = std::env::temp_dir().join(format!("earfun_stage1_cfg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        // Corrupt settings file is backed up before any save overwrites it
        let corrupt_payload = "{corrupt_user_presets: [1,2,3]";
        std::fs::write(&path, corrupt_payload).unwrap();
        let mut settings = Settings::new(Some(&path));
        assert!(settings.load_warning.is_some());
        let backup_path = dir.join("settings.json.corrupt.bak");
        assert_eq!(
            std::fs::read_to_string(&backup_path).unwrap(),
            corrupt_payload
        );

        // Theme preference round-trips and defaults to Light for legacy configs
        assert_eq!(settings.data.theme, settings::ThemeMode::Light);
        settings.data.theme = settings::ThemeMode::Dark;
        settings.save().unwrap();
        let reloaded = Settings::new(Some(&path));
        assert_eq!(reloaded.data.theme, settings::ThemeMode::Dark);
        assert!(reloaded.load_warning.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_stage1_preset_name_sanitization_and_audio_security() {
        // Control characters, bidi overrides, and zero-width/line separators must be rejected
        assert!(presets::sanitize_preset_name("Clean Vi Name").is_ok());
        assert!(presets::sanitize_preset_name("Bad\nName").is_err());
        assert!(presets::sanitize_preset_name("Bad\u{202E}Name").is_err());
        assert!(presets::sanitize_preset_name("Bad\u{200B}Name").is_err());
        assert!(presets::sanitize_preset_name("Bad\u{2060}Name").is_err());
        assert!(presets::sanitize_preset_name("Bad\u{FEFF}Name").is_err());
        assert!(presets::sanitize_preset_name("Bad\u{2028}Name").is_err());
        assert!(presets::sanitize_preset_name("Bad\u{2029}Name").is_err());
        assert!(presets::sanitize_preset_name(&"x".repeat(101)).is_err());

        // Non-numeric PipeWire sink IDs must never be returned (prevents wpctl option injection)
        let malicious = serde_json::json!([{
            "id": "--help",
            "info": {
                "props": {
                    "node.name": "bluez_output.AA_BB_CC_DD_EE_FF.1",
                    "media.class": "Audio/Sink"
                }
            }
        }]);
        assert_eq!(audio::find_sink(&malicious, "AA:BB:CC:DD:EE:FF"), None);
        assert_eq!(audio::find_sink(&malicious, "GG:HH:II:JJ:KK:LL"), None);

        // External target validation blocks cmd injection, arbitrary schemes, and non-github HTTPS
        assert!(audio::validate_external_target("ms-settings:bluetooth").is_ok());
        assert!(audio::validate_external_target("ms-settings:sound").is_ok());
        assert!(audio::validate_external_target("ms-settings:apps-volume").is_ok());
        assert!(audio::validate_external_target("https://github.com/lnb2x/earfun-studio").is_ok());
        assert!(audio::validate_external_target("https://evil.example/repo").is_err());
        assert!(audio::validate_external_target("https://github.com.evil.example/repo").is_err());
        assert!(audio::validate_external_target("https://user@github.com/repo").is_err());
        assert!(audio::validate_external_target("https://evil.example/a&calc.exe").is_err());
        assert!(audio::validate_external_target("file:///C:/Windows/System32/cmd.exe").is_err());
        assert!(audio::validate_external_target("cmd /c calc").is_err());
    }

    #[tokio::test]
    async fn test_stage2_short_request_and_oversized_direct_never_panic() {
        let (core, _client) = setup_test_core();
        assert!(core.request(&[0x00, 0x0a]).await.is_err());
        assert!(core.direct(20, &vec![0u8; 600]).await.is_err());
    }

    #[tokio::test]
    async fn test_stage2_stop_find_timer_completes_without_self_abort() {
        struct YieldingClient {
            writes: Mutex<Vec<Vec<u8>>>,
        }
        impl GattClient for YieldingClient {
            fn is_connected(&self) -> bool {
                true
            }
            fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    self.writes.lock().unwrap().push(packet);
                    Ok(())
                })
            }
            fn close_session(&self) -> BoxFuture<'_, ()> {
                Box::pin(async {})
            }
        }

        let core = BackendCore::new(Arc::new(|_| {}));
        *core.step_sleep.lock().unwrap() = Duration::from_millis(1);
        *core.find_duration.lock().unwrap() = Duration::from_millis(20);
        core.state.lock().unwrap().version = Some("TW600-0.4.9".to_string());
        let client = Arc::new(YieldingClient {
            writes: Mutex::new(Vec::new()),
        });
        *core.client.lock().unwrap() = Some(client.clone());

        core.find(1).await.unwrap();
        assert!(core.finding.load(Ordering::SeqCst));

        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(
            !core.finding.load(Ordering::SeqCst),
            "10s timer task must finish sending stop_find without aborting itself"
        );
        let writes = client.writes.lock().unwrap().clone();
        assert_eq!(
            writes,
            vec![
                p::hex_decode("000a033201").unwrap(),
                p::hex_decode("000a033200").unwrap(),
            ]
        );
    }

    #[tokio::test]
    async fn test_stage2_codecs_and_touch_lock_retry_until_all_fields_match() {
        struct LaggingMultiFieldClient {
            core: Weak<BackendCore>,
            codec_queries: Mutex<usize>,
            lock_queries: Mutex<usize>,
        }
        impl GattClient for LaggingMultiFieldClient {
            fn is_connected(&self) -> bool {
                true
            }
            fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
                Box::pin(async move {
                    let cmd = packet[3];
                    if cmd == 47 {
                        let mut q = self.codec_queries.lock().unwrap();
                        *q += 1;
                        // Query 1 returns stale aptx=true (bits=3); Query 2 returns updated aptx=false (bits=2)
                        let bits = if *q == 1 { 3u8 } else { 2u8 };
                        if let Some(c) = self.core.upgrade() {
                            c.notify(&p::packet(10, 0x832f, &[0, bits]).unwrap());
                        }
                    } else if cmd == 53 {
                        let mut q = self.lock_queries.lock().unwrap();
                        *q += 1;
                        // Query 1 returns dis_l=false, dis_r=false (0); Query 2 returns dis_l=false, dis_r=true (3)
                        let val = if *q == 1 { 0u8 } else { 3u8 };
                        if let Some(c) = self.core.upgrade() {
                            c.notify(&p::packet(10, 0x8335, &[0, val]).unwrap());
                        }
                    }
                    Ok(())
                })
            }
            fn close_session(&self) -> BoxFuture<'_, ()> {
                Box::pin(async {})
            }
        }

        let core = BackendCore::new(Arc::new(|_| {}));
        *core.step_sleep.lock().unwrap() = Duration::from_millis(1);
        *core.state.lock().unwrap() = DeviceState {
            version: Some("TW600-0.4.9".to_string()),
            dual: Some(false),
            ldac: Some(false),
            aptx: Some(true),
            adaptive: Some(true),
            lossless: Some(false),
            dis_l: Some(false),
            dis_r: Some(false),
            ..Default::default()
        };
        let client = Arc::new(LaggingMultiFieldClient {
            core: Arc::downgrade(&core),
            codec_queries: Mutex::new(0),
            lock_queries: Mutex::new(0),
        });
        *core.client.lock().unwrap() = Some(client);

        let mut wanted = BTreeMap::new();
        wanted.insert("ldac".to_string(), false); // ldac unchanged!
        wanted.insert("aptx".to_string(), false); // aptx changed!
        wanted.insert("adaptive".to_string(), true);
        wanted.insert("lossless".to_string(), false);

        core.codecs(&wanted).await.unwrap();
        assert_eq!(core.state.lock().unwrap().aptx, Some(false));

        core.disable_touch(false, true).await.unwrap();
        assert_eq!(core.state.lock().unwrap().dis_r, Some(true));
    }

    #[tokio::test]
    async fn test_stage2_streamed_phones_collects_all_entries_before_returning() {
        struct StreamPhonesClient {
            core: Weak<BackendCore>,
        }
        impl GattClient for StreamPhonesClient {
            fn is_connected(&self) -> bool {
                true
            }
            fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
                let weak = self.core.clone();
                Box::pin(async move {
                    if packet[3] == 44 {
                        tokio::spawn(async move {
                            for (num, name, mac) in
                                [(0u8, b"Laptop\0", "aabbcc"), (1u8, b"Phone\0\0", "112233")]
                            {
                                tokio::task::yield_now().await;
                                let mut raw = p::hex_decode("000a032c00").unwrap();
                                raw.push(num);
                                let mut name_buf = name.to_vec();
                                name_buf.resize(19, 0);
                                raw.extend_from_slice(&name_buf);
                                raw.extend_from_slice(b"\0\0");
                                raw.extend_from_slice(&p::hex_decode(mac).unwrap());
                                raw.push(1);
                                if let Some(c) = weak.upgrade() {
                                    c.notify(&raw);
                                }
                            }
                        });
                    }
                    Ok(())
                })
            }
            fn close_session(&self) -> BoxFuture<'_, ()> {
                Box::pin(async {})
            }
        }

        let core = BackendCore::new(Arc::new(|_| {}));
        *core.step_sleep.lock().unwrap() = Duration::from_millis(15);
        core.state.lock().unwrap().version = Some("TW600-0.4.9".to_string());
        *core.client.lock().unwrap() = Some(Arc::new(StreamPhonesClient {
            core: Arc::downgrade(&core),
        }));

        core.phones_action(44, None).await.unwrap();
        assert_eq!(core.phones.lock().unwrap().len(), 2);
        // Trying to connect a 3rd phone when 2 are already connected must be blocked immediately
        assert!(core.phones_action(41, Some(2)).await.is_err());
    }

    #[tokio::test]
    async fn run_plan_reconnect_timeout_resets_to_disconnected() {
        struct RebootDropClient {
            core: Weak<BackendCore>,
            connected: AtomicBool,
        }

        impl GattClient for RebootDropClient {
            fn is_connected(&self) -> bool {
                self.connected.load(Ordering::SeqCst)
            }
            fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
                Box::pin(async move {
                    let cmd = packet[3];
                    if cmd == 39 {
                        self.connected.store(false, Ordering::SeqCst);
                        return Ok(());
                    }
                    if let Some(c) = self.core.upgrade() {
                        let raw = match cmd {
                            38 => p::packet(10, 0x8326, &[0, 1]).unwrap(),
                            47 => p::packet(10, 0x832f, &[0, 19]).unwrap(),
                            _ => p::packet(10, 0x8300 | (cmd as u16), &[0, 0]).unwrap(),
                        };
                        c.notify(&raw);
                    }
                    Ok(())
                })
            }
            fn close_session(&self) -> BoxFuture<'_, ()> {
                Box::pin(async move {
                    self.connected.store(false, Ordering::SeqCst);
                })
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let core = BackendCore::new(Arc::new(move |ev| captured.lock().unwrap().push(ev)));
        *core.step_sleep.lock().unwrap() = Duration::from_millis(10);
        *core.restart_deadline.lock().unwrap() = Duration::from_millis(200);
        *core.address.lock().unwrap() = Some("AA:BB:CC:DD:EE:FF".to_string());

        let init_state = DeviceState {
            version: Some("TW600-test".to_string()),
            dual: Some(true),
            ldac: Some(false),
            aptx: Some(true),
            adaptive: Some(true),
            lossless: Some(true),
            ..Default::default()
        };
        *core.state.lock().unwrap() = init_state.clone();
        *core.connection_status.lock().unwrap() = backend::ConnectionStatus::Connected;
        *core.client.lock().unwrap() = Some(Arc::new(RebootDropClient {
            core: Arc::downgrade(&core),
            connected: AtomicBool::new(true),
        }));

        let hook_called = Arc::new(AtomicBool::new(false));
        let hook_called_flag = hook_called.clone();
        let weak_core = Arc::downgrade(&core);
        core.set_reconnect_hook(Arc::new(move |_addr, _full| {
            hook_called_flag.store(true, Ordering::SeqCst);
            let weak_core = weak_core.clone();
            Box::pin(async move {
                if let Some(c) = weak_core.upgrade() {
                    c.set_connection_status(backend::ConnectionStatus::Connecting);
                    c.set_phase(backend::ConnectionPhase::OpeningBle);
                }
                std::future::pending::<()>().await;
                Ok(())
            })
        }));

        let mut wanted = BTreeMap::new();
        for &k in workflows::CODECS {
            wanted.insert(k.to_string(), init_state.get_bool(k).unwrap());
        }
        wanted.insert("ldac".to_string(), true);
        let plan = workflows::codecs(&init_state, &wanted).unwrap();

        let err = core.run_plan(&plan).await.unwrap_err();
        assert!(hook_called.load(Ordering::SeqCst));
        assert!(err.to_string().contains("Tắt kết nối hai thiết bị"));
        assert_eq!(
            *core.connection_status.lock().unwrap(),
            backend::ConnectionStatus::Disconnected
        );
        assert!(core.client.lock().unwrap().is_none());
        let last_conn = events.lock().unwrap().iter().rev().find_map(|ev| match ev {
            backend::BackendEvent::Connection(status) => Some(status.clone()),
            _ => None,
        });
        assert_eq!(last_conn, Some(backend::ConnectionStatus::Disconnected));
    }

    #[tokio::test]
    async fn run_plan_restores_polling_after_reconnect() {
        struct RebootSimClient {
            core: Weak<BackendCore>,
            connected: AtomicBool,
            dual: Arc<AtomicBool>,
        }

        impl GattClient for RebootSimClient {
            fn is_connected(&self) -> bool {
                self.connected.load(Ordering::SeqCst)
            }
            fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
                Box::pin(async move {
                    let cmd = packet[3];
                    if cmd == 39 {
                        self.dual.store(packet[4] != 0, Ordering::SeqCst);
                        self.connected.store(false, Ordering::SeqCst);
                        if let Some(c) = self.core.upgrade() {
                            c.disconnect_internal().await;
                        }
                        return Ok(());
                    }
                    if let Some(c) = self.core.upgrade() {
                        let raw = match cmd {
                            38 => p::packet(
                                10,
                                0x8326,
                                &[0, u8::from(self.dual.load(Ordering::SeqCst))],
                            )
                            .unwrap(),
                            47 => p::packet(10, 0x832f, &[0, 19]).unwrap(),
                            74 => p::packet(10, 0x834a, &[0, 0]).unwrap(),
                            _ => p::packet(10, 0x8300 | (cmd as u16), &[0, 0]).unwrap(),
                        };
                        c.notify(&raw);
                    }
                    Ok(())
                })
            }
            fn close_session(&self) -> BoxFuture<'_, ()> {
                Box::pin(async move {
                    self.connected.store(false, Ordering::SeqCst);
                })
            }
        }

        let core = BackendCore::new(Arc::new(|_| {}));
        *core.step_sleep.lock().unwrap() = Duration::from_millis(1);
        *core.address.lock().unwrap() = Some("AA:BB:CC:DD:EE:FF".to_string());
        let dual_flag = Arc::new(AtomicBool::new(true));

        let init_state = DeviceState {
            version: Some("TW600-test".to_string()),
            dual: Some(true),
            ldac: Some(false),
            aptx: Some(true),
            adaptive: Some(true),
            lossless: Some(true),
            ..Default::default()
        };
        *core.state.lock().unwrap() = init_state.clone();
        *core.client.lock().unwrap() = Some(Arc::new(RebootSimClient {
            core: Arc::downgrade(&core),
            connected: AtomicBool::new(true),
            dual: dual_flag.clone(),
        }));

        let weak_core = Arc::downgrade(&core);
        let dual_for_reconn = dual_flag.clone();
        let state_for_reconn = init_state.clone();
        core.set_reconnect_hook(Arc::new(move |_addr, full| {
            assert!(!full);
            let weak_core = weak_core.clone();
            let dual_for_reconn = dual_for_reconn.clone();
            let mut st = state_for_reconn.clone();
            st.dual = Some(dual_for_reconn.load(Ordering::SeqCst));
            Box::pin(async move {
                if let Some(c) = weak_core.upgrade() {
                    *c.state.lock().unwrap() = st;
                    *c.client.lock().unwrap() = Some(Arc::new(RebootSimClient {
                        core: Arc::downgrade(&c),
                        connected: AtomicBool::new(true),
                        dual: dual_for_reconn,
                    }));
                }
                Ok(())
            })
        }));

        let weak_core2 = Arc::downgrade(&core);
        let dual_for_query = dual_flag.clone();
        let state_for_query = init_state.clone();
        core.set_query_hook(Arc::new(move |_full| {
            let weak_core2 = weak_core2.clone();
            let dual_for_query = dual_for_query.clone();
            let mut st = state_for_query.clone();
            st.dual = Some(dual_for_query.load(Ordering::SeqCst));
            Box::pin(async move {
                if let Some(c) = weak_core2.upgrade() {
                    *c.state.lock().unwrap() = st;
                }
                Ok(())
            })
        }));

        let plan = workflows::dual(&init_state, false).unwrap();
        assert!(core.poll_abort.lock().unwrap().is_none());
        core.run_plan(&plan).await.unwrap();
        assert!(
            core.poll_abort.lock().unwrap().is_some(),
            "Polling task must be restored after run_plan reconnects"
        );
        core.disconnect().await;
    }

    #[tokio::test]
    async fn stop_find_is_not_blocked_by_op_lock() {
        let (core, client) = setup_test_core();
        core.finding.store(true, Ordering::SeqCst);
        let _op_guard = core.op_lock.lock().await;
        tokio::time::timeout(Duration::from_secs(1), core.find(0))
            .await
            .expect("stop_find must not wait behind op_lock")
            .unwrap();
        assert!(!core.finding.load(Ordering::SeqCst));
        assert!(client
            .writes
            .lock()
            .unwrap()
            .contains(&p::request(50, &[0])));
    }

    #[test]
    fn oversized_settings_is_backed_up_and_preserved_across_save() {
        let dir = std::env::temp_dir().join(format!("earfun_oversized_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let oversized = vec![b'x'; 1_048_600];
        std::fs::write(&path, &oversized).unwrap();

        let mut settings = Settings::new(Some(&path));
        assert!(settings.load_warning.is_some());
        assert!(!settings.save_blocked);

        let backup = dir.join("settings.json.corrupt.bak");
        assert!(backup.exists());
        assert_eq!(std::fs::read(&backup).unwrap().len(), oversized.len());

        settings.data.theme = settings::ThemeMode::Dark;
        settings.save().unwrap();
        assert_eq!(std::fs::read(&backup).unwrap().len(), oversized.len());
        assert_eq!(
            Settings::new(Some(&path)).data.theme,
            settings::ThemeMode::Dark
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_corrupt_backup_is_not_overwritten_and_bad_types_warn() {
        let dir = std::env::temp_dir().join(format!("earfun_bak_keep_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let primary_bak = dir.join("settings.json.corrupt.bak");

        std::fs::write(&primary_bak, "first-corrupt-backup").unwrap();
        std::fs::write(
            &path,
            serde_json::json!({
                "last_mac": 12345,
                "theme": "sepia"
            })
            .to_string(),
        )
        .unwrap();

        let mut settings = Settings::new(Some(&path));
        assert!(settings.load_warning.is_some());
        assert_eq!(
            std::fs::read_to_string(&primary_bak).unwrap(),
            "first-corrupt-backup"
        );
        let extra_backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("settings.json.corrupt-") && name.ends_with(".bak")
            })
            .collect();
        assert_eq!(extra_backups.len(), 1);

        settings.save_blocked = true;
        assert!(settings.save().is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn saved_settings_file_uses_0600_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("earfun_mode_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let settings = Settings::new(Some(&path));
        settings.save().unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
