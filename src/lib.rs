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
        let mut st = DeviceState::default();
        st.dual = Some(true);
        st.ldac = Some(false);
        st.aptx = Some(true);
        st.adaptive = Some(true);
        st.lossless = Some(true);

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

        let mut le_state = DeviceState::default();
        le_state.le_audio = Some(true);
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
                    let mut dev = self.device.lock().unwrap();
                    if cmd == 39 || cmd == 46 {
                        let val = packet[4];
                        dev.writes.push((cmd, val));
                        if cmd == 39 {
                            dev.state.dual = Some(val != 0);
                            self.connected.store(false, Ordering::SeqCst);
                            drop(dev);
                            if let Some(core) = self.core.upgrade() {
                                core.lost();
                            }
                        } else if !dev.fail_codec {
                            dev.state.aptx = Some((val & 1) != 0);
                            dev.state.adaptive = Some((val & 2) != 0);
                            dev.state.ldac = Some((val & 8) != 0);
                            dev.state.lossless = Some((val & 16) != 0);
                        }
                        return Ok(());
                    }
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
                    drop(dev);
                    if let Some(core) = self.core.upgrade() {
                        let raw = p::packet(10, 0x8300 | (cmd as u16), &[0, value]).unwrap();
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

        let mut init_state = DeviceState::default();
        init_state.version = Some("TW600-test".to_string());
        init_state.dual = Some(true);
        init_state.ldac = Some(false);
        init_state.aptx = Some(true);
        init_state.adaptive = Some(true);
        init_state.lossless = Some(true);

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
}
