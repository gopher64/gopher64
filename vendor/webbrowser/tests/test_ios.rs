#[cfg(target_os = "macos")]
mod common;

#[cfg(target_os = "macos")]
mod tests {
    const TEST_PLATFORM: &str = "ios";

    use super::common::check_request_received_using;
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use webbrowser::Browser;

    // to run this test, run it as:
    // cargo test --test test_ios -- --ignored
    //
    // MAKE SURE: an iOS simulator instance is already running
    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ios() {
        let uri = format!("/{}", TEST_PLATFORM);
        let ipv4 = get_ipv4_address();

        let mut app_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        app_dir.push("tests/test-ios-app");

        // build test glue code
        let mut glue_dir = PathBuf::from(&app_dir);
        glue_dir.push("testglue");
        run_cmd(&glue_dir, "glue code build failed", &["./build"]);

        // invoke server
        check_request_received_using(uri, &ipv4, |url, _port| {
            // modify ios app code to use the correct url
            let mut swift_src = PathBuf::from(&app_dir);
            swift_src.push("test-ios-app/ContentView.swift");
            let old_code =
                fs::read_to_string(&swift_src).expect("failed to read ContentView.swift");
            let new_code = old_code
                .split('\n')
                .map(|s| {
                    if s.starts_with("let SERVER_URL") {
                        format!("let SERVER_URL = \"{}\"", url)
                    } else {
                        s.into()
                    }
                })
                .collect::<Vec<String>>()
                .join("\n");
            fs::write(&swift_src, new_code).expect("failed to modify ContentView.swift");

            // build app
            run_cmd(
                &app_dir,
                "failed to build ios app",
                &[
                    "xcrun",
                    "xcodebuild",
                    "-project",
                    "test-ios-app.xcodeproj",
                    "-scheme",
                    "test-ios-app",
                    "-configuration",
                    "Debug",
                    "-destination",
                    "platform=iOS Simulator,name=iphone-latest",
                    "-derivedDataPath",
                    "build",
                ],
            );

            // launch app on simulator
            run_cmd(
                &app_dir,
                "failed to install app on simulator",
                &[
                    "xcrun",
                    "simctl",
                    "install",
                    "booted",
                    "build/Build/Products/Debug-iphonesimulator/test-ios-app.app",
                ],
            );
            run_cmd(
                &app_dir,
                "failed to launch app on simulator",
                &[
                    "xcrun",
                    "simctl",
                    "launch",
                    "booted",
                    "in.rootnet.webbrowser.test-ios-app",
                ],
            );

            // revert to the old code
            fs::write(&swift_src, &old_code).expect("failed to modify ContentView.swift");
        })
        .await;
    }

    fn get_ipv4_address() -> String {
        let output = Command::new("sh")
            .arg("-c")
            .arg("ifconfig | grep 'inet ' | awk '{ print $2 }' | grep -v ^127.0.0")
            .output()
            .expect("failed to get non-local ipv4 address");
        std::str::from_utf8(&output.stdout)
            .expect("unable to parse output into utf8")
            .split('\n')
            .next()
            .expect("no ip address found")
            .into()
    }

    #[test]
    fn test_existence_default() {
        assert!(Browser::is_available(), "should have found a browser");
    }

    fn run_cmd(app_dir: &PathBuf, failure_msg: &str, args: &[&str]) {
        let _ = Command::new(args[0])
            .args(&args[1..])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .current_dir(app_dir)
            .status()
            .expect(failure_msg);
    }
}
