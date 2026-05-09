use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use which::which;

use crate::host::{BuildOptions, Host, HostPaths, remove_generated_path, workspace_root};

const PROJECT_NAME: &str = "GPUIiOS";
const BUNDLE_ID: &str = "dev.glasshq.GPUIiOS";
const DEFAULT_TEAM_ID: &str = "DA7B5U47PT";
const DEFAULT_LOG_PORT: u16 = 9632;

#[derive(Debug)]
pub struct IosHost {
    paths: HostPaths,
}

#[derive(Clone, Debug)]
struct Device {
    core_id: String,
    legacy_udid: String,
    name: String,
    model: String,
    os_version: String,
    status: DeviceStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeviceStatus {
    Available,
    Reachable,
    Unavailable,
}

#[derive(Debug)]
pub struct BuildResult {
    app_path: PathBuf,
    simulator_udid: Option<String>,
    device: Option<Device>,
}

struct ChildGuard {
    child: Option<Child>,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl IosHost {
    pub fn load() -> Result<Self> {
        let workspace_root = workspace_root()?;
        let host_root = workspace_root.join("hosts").join("ios");
        Ok(Self {
            paths: HostPaths {
                workspace_root,
                host_root,
            },
        })
    }

    fn project_spec(&self) -> PathBuf {
        self.paths.host_root.join("project.yml")
    }

    fn xcodeproj(&self) -> PathBuf {
        self.paths
            .host_root
            .join(format!("{PROJECT_NAME}.xcodeproj"))
    }

    fn derived_data(&self, sim: bool) -> PathBuf {
        self.paths
            .host_root
            .join("build")
            .join(if sim { "simulator" } else { "device" })
    }

    fn project_name(&self) -> &'static str {
        PROJECT_NAME
    }

    fn bundle_id(&self) -> &'static str {
        BUNDLE_ID
    }

    fn development_team(&self) -> String {
        env::var("DEVELOPMENT_TEAM").unwrap_or_else(|_| DEFAULT_TEAM_ID.to_string())
    }

    fn log_port(&self) -> u16 {
        env::var("GPUI_LOG_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_LOG_PORT)
    }
}

fn print_demos() {
    println!("Available iOS demos:\n");
    for demo in gpui_examples::ios::IOS_DEMOS {
        println!("  {:<18} {}", demo.name, demo.description);
    }
    println!("\nRun one with `cargo gpui run ios <demo>`.");
}

fn is_known_demo(name: &str) -> bool {
    gpui_examples::ios::IOS_DEMOS
        .iter()
        .any(|demo| demo.name == name)
}

impl Host for IosHost {
    fn sync(&self) -> Result<()> {
        ensure_tool("xcodegen", "Install it with `brew install xcodegen`.")?;
        remove_generated_path(&self.xcodeproj())?;
        let status = Command::new("xcodegen")
            .arg("generate")
            .arg("--spec")
            .arg(self.project_spec())
            .current_dir(&self.paths.host_root)
            .status()
            .context("failed to run xcodegen")?;
        ensure_success(status, "xcodegen generate failed")
    }

    fn list_devices(&self) -> Result<()> {
        ensure_devicectl()?;
        let devices = fetch_physical_devices()?;
        println!("Connected iOS devices:\n");
        if devices.is_empty() {
            println!("  (none found)");
            bail!("no physical iOS devices found");
        }

        for device in devices {
            println!(
                "  {}  {}  iOS {}  ({})",
                device.name,
                device.model,
                device.os_version,
                device.status.as_str()
            );
            println!("    {}", device.core_id);
            println!("    {}", device.legacy_udid);
            println!();
        }
        Ok(())
    }

    fn build(&self, options: &BuildOptions) -> Result<()> {
        self.build_inner(options).map(|_| ())
    }

    fn run(&self, demo: Option<&str>, options: &BuildOptions) -> Result<()> {
        let Some(demo_name) = demo else {
            print_demos();
            return Ok(());
        };

        if !is_known_demo(demo_name) {
            bail!("unknown iOS demo `{demo_name}`");
        }

        let build = self.build_inner(options)?;
        if options.sim {
            let simulator_udid = build
                .simulator_udid
                .as_deref()
                .context("missing booted simulator after build")?;
            ensure_simulator_ui(simulator_udid)?;
            let terminate_status = Command::new("xcrun")
                .args(["simctl", "terminate", simulator_udid, self.bundle_id()])
                .status();
            if let Ok(status) = terminate_status {
                let _ = status;
            }

            let install_status = Command::new("xcrun")
                .args(["simctl", "install", simulator_udid])
                .arg(&build.app_path)
                .status()
                .context("failed to install app on simulator")?;
            ensure_success(install_status, "simulator install failed")?;

            println!("Launching {demo_name} on simulator {simulator_udid}...");
            let launch_status = Command::new("xcrun")
                .args([
                    "simctl",
                    "launch",
                    "--console",
                    simulator_udid,
                    self.bundle_id(),
                    demo_name,
                ])
                .env("SIMCTL_CHILD_GPUI_IOS_DEMO", demo_name)
                .status()
                .context("failed to launch app on simulator")?;
            ensure_success(launch_status, "simulator launch failed")
        } else {
            let device = build
                .device
                .as_ref()
                .context("missing device metadata after build")?;
            let mut log_listener = spawn_log_listener(self.log_port())?;
            install_on_device(device, &build.app_path)?;

            println!("Launching {demo_name} on {}...", device.name);
            let output = Command::new("xcrun")
                .args([
                    "devicectl",
                    "device",
                    "process",
                    "launch",
                    "--terminate-existing",
                    "--device",
                    &device.core_id,
                    self.bundle_id(),
                    demo_name,
                ])
                .env("DEVICECTL_CHILD_GPUI_IOS_DEMO", demo_name)
                .output()
                .context("failed to launch app on device")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let combined = format!("{stdout}{stderr}");
                if combined.to_lowercase().contains("locked") {
                    bail!("device is locked; unlock it and try again\n{combined}");
                }
                bail!("device launch failed\n{combined}");
            }

            print!("{}", String::from_utf8_lossy(&output.stdout));
            print!("{}", String::from_utf8_lossy(&output.stderr));
            println!("\n--- Streaming logs (Ctrl+C to stop) ---\n");
            if let Some(child) = log_listener.child.as_mut() {
                let _ = child.wait();
            }
            log_listener.child = None;
            Ok(())
        }
    }

    fn build_rust(&self) -> Result<()> {
        build_rust_inner(self)
    }
}

impl IosHost {
    fn build_inner(&self, options: &BuildOptions) -> Result<BuildResult> {
        self.sync()?;
        if options.sim {
            let simulator_udid = ensure_booted_simulator()?;
            let build_dir = self.derived_data(true);
            fs::create_dir_all(&build_dir).with_context(|| {
                format!(
                    "failed to create simulator build directory {}",
                    build_dir.display()
                )
            })?;
            let status = Command::new("xcodebuild")
                .arg("-project")
                .arg(self.xcodeproj())
                .arg("-scheme")
                .arg(self.project_name())
                .arg("-configuration")
                .arg(configuration_name(options.release))
                .arg("-destination")
                .arg(format!("id={simulator_udid}"))
                .arg("-derivedDataPath")
                .arg(&build_dir)
                .arg("DEVELOPMENT_TEAM=".to_owned() + &self.development_team())
                .arg("build")
                .current_dir(&self.paths.host_root)
                .status()
                .context("failed to run xcodebuild for simulator")?;
            ensure_success(status, "simulator build failed")?;

            let app_path = build_dir.join("Build/Products").join(format!(
                "{}-iphonesimulator/{PROJECT_NAME}.app",
                configuration_name(options.release)
            ));
            if !app_path.is_dir() {
                bail!("built simulator app not found at {}", app_path.display());
            }

            Ok(BuildResult {
                app_path,
                simulator_udid: Some(simulator_udid),
                device: None,
            })
        } else {
            ensure_devicectl()?;
            let selected_device = select_device(options.device.as_deref())?;
            let build_dir = self.derived_data(false);
            fs::create_dir_all(&build_dir).with_context(|| {
                format!(
                    "failed to create device build directory {}",
                    build_dir.display()
                )
            })?;
            let status = Command::new("xcodebuild")
                .arg("-project")
                .arg(self.xcodeproj())
                .arg("-scheme")
                .arg(self.project_name())
                .arg("-configuration")
                .arg(configuration_name(options.release))
                .arg("-destination")
                .arg(format!("id={}", selected_device.legacy_udid))
                .arg("-derivedDataPath")
                .arg(&build_dir)
                .arg("-allowProvisioningUpdates")
                .arg("DEVELOPMENT_TEAM=".to_owned() + &self.development_team())
                .arg("CODE_SIGN_STYLE=Automatic")
                .arg("build")
                .current_dir(&self.paths.host_root)
                .status()
                .context("failed to run xcodebuild for device")?;
            ensure_success(status, "device build failed")?;

            let app_path = build_dir.join("Build/Products").join(format!(
                "{}-iphoneos/{PROJECT_NAME}.app",
                configuration_name(options.release)
            ));
            if !app_path.is_dir() {
                bail!("built device app not found at {}", app_path.display());
            }

            Ok(BuildResult {
                app_path,
                simulator_udid: None,
                device: Some(selected_device),
            })
        }
    }
}

fn build_rust_inner(host: &IosHost) -> Result<()> {
    let rustup = env::var("RUSTUP_BIN")
        .ok()
        .map(PathBuf::from)
        .or_else(|| which("rustup").ok())
        .context("rustup is required; install it with `brew install rustup-init && rustup-init`")?;

    let cargo = env::var("CARGO_BIN")
        .ok()
        .map(PathBuf::from)
        .or_else(|| rustup_which(&rustup, "cargo").ok())
        .context("cargo is missing from the active Rust toolchain")?;

    let rustc = rustup_which(&rustup, "rustc")
        .context("no Rust toolchain is installed; run `rustup toolchain install stable`")?;

    let platform_name = env::var("PLATFORM_NAME").unwrap_or_default();
    let built_products_dir =
        PathBuf::from(env::var("BUILT_PRODUCTS_DIR").context("BUILT_PRODUCTS_DIR is required")?);
    let release = env::var("CONFIGURATION")
        .map(|value| value == "Release")
        .unwrap_or(false);
    let profile_dir = if release { "release" } else { "debug" };

    let log_relay = env::var("GPUI_LOG_RELAY").ok().or_else(|| {
        detect_host_ip().map(|ip| {
            format!(
                "{ip}:{}",
                env::var("GPUI_LOG_PORT")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_LOG_PORT)
            )
        })
    });

    if let Some(target) = log_relay.as_deref() {
        println!("Log relay target: {target}");
    } else {
        println!("Log relay: disabled (no local network IP detected)");
    }

    match platform_name.as_str() {
        "iphoneos" => {
            build_target(
                host,
                &rustup,
                &cargo,
                &rustc,
                "aarch64-apple-ios",
                release,
                log_relay.as_deref(),
            )?;
            let source = host
                .paths
                .workspace_root
                .join("target/aarch64-apple-ios")
                .join(profile_dir)
                .join("libgpui_ios.a");
            copy_static_lib(&source, &built_products_dir.join("libgpui_ios.a"))?;
        }
        "iphonesimulator" => {
            build_target(
                host,
                &rustup,
                &cargo,
                &rustc,
                "aarch64-apple-ios-sim",
                release,
                log_relay.as_deref(),
            )?;
            let arm64 = host
                .paths
                .workspace_root
                .join("target/aarch64-apple-ios-sim")
                .join(profile_dir)
                .join("libgpui_ios.a");

            if host_architecture()?.trim() == "arm64" {
                copy_static_lib(&arm64, &built_products_dir.join("libgpui_ios.a"))?;
            } else {
                build_target(
                    host,
                    &rustup,
                    &cargo,
                    &rustc,
                    "x86_64-apple-ios",
                    release,
                    log_relay.as_deref(),
                )?;
                let x64 = host
                    .paths
                    .workspace_root
                    .join("target/x86_64-apple-ios")
                    .join(profile_dir)
                    .join("libgpui_ios.a");
                let status = Command::new("lipo")
                    .args(["-create", "-output"])
                    .arg(built_products_dir.join("libgpui_ios.a"))
                    .arg(&arm64)
                    .arg(&x64)
                    .status()
                    .context("failed to run lipo")?;
                ensure_success(status, "failed to create universal simulator library")?;
            }
        }
        other => bail!("unsupported PLATFORM_NAME={other}"),
    }

    Ok(())
}

fn configuration_name(release: bool) -> &'static str {
    if release { "Release" } else { "Debug" }
}

fn ensure_tool(tool: &str, install_hint: &str) -> Result<()> {
    if which(tool).is_err() {
        bail!("{tool} is required. {install_hint}");
    }
    Ok(())
}

fn ensure_devicectl() -> Result<()> {
    let status = Command::new("xcrun")
        .args(["devicectl", "--help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to query xcrun devicectl")?;
    if !status.success() {
        bail!("xcrun devicectl is required (Xcode 15+)");
    }
    Ok(())
}

fn ensure_success(status: ExitStatus, context: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("{context}"))
    }
}

fn ensure_booted_simulator() -> Result<String> {
    if let Some(udid) = booted_simulator_udid()? {
        return Ok(udid);
    }

    println!("No booted simulator found. Booting a default iPhone...");
    let json = command_json(
        Command::new("xcrun").args(["simctl", "list", "devices", "available", "-j"]),
        "failed to list available simulators",
    )?;
    let devices_map = json
        .get("devices")
        .and_then(Value::as_object)
        .context("simctl output missing devices map")?;

    let mut runtimes = devices_map.keys().cloned().collect::<Vec<_>>();
    runtimes.sort();
    runtimes.reverse();

    let mut selected = None;
    for runtime in runtimes {
        if !runtime.contains("iOS") {
            continue;
        }
        if let Some(devices) = devices_map.get(&runtime).and_then(Value::as_array) {
            for device in devices {
                if device.get("isAvailable").and_then(Value::as_bool) != Some(true) {
                    continue;
                }
                let name = device
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !name.contains("iPhone") {
                    continue;
                }
                selected = device
                    .get("udid")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if selected.is_some() {
                    break;
                }
            }
        }
        if selected.is_some() {
            break;
        }
    }

    let udid = selected.context("no iPhone simulator available; create one in Xcode")?;
    let status = Command::new("xcrun")
        .args(["simctl", "boot", &udid])
        .status()
        .context("failed to boot simulator")?;
    ensure_success(status, "failed to boot simulator")?;
    Ok(udid)
}

fn ensure_simulator_ui(udid: &str) -> Result<()> {
    let open_status = Command::new("open")
        .args(["-a", "Simulator"])
        .status()
        .context("failed to open Simulator.app")?;
    ensure_success(open_status, "failed to open Simulator.app")?;

    let bootstatus = Command::new("xcrun")
        .args(["simctl", "bootstatus", udid, "-b"])
        .status()
        .context("failed to wait for simulator boot status")?;
    ensure_success(bootstatus, "simulator failed to finish booting")
}

fn booted_simulator_udid() -> Result<Option<String>> {
    let json = command_json(
        Command::new("xcrun").args(["simctl", "list", "devices", "booted", "-j"]),
        "failed to list booted simulators",
    )?;
    let devices_map = json
        .get("devices")
        .and_then(Value::as_object)
        .context("simctl output missing devices map")?;
    for devices in devices_map.values() {
        if let Some(entries) = devices.as_array() {
            for entry in entries {
                if entry.get("state").and_then(Value::as_str) == Some("Booted") {
                    if let Some(udid) = entry.get("udid").and_then(Value::as_str) {
                        return Ok(Some(udid.to_string()));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn fetch_physical_devices() -> Result<Vec<Device>> {
    let output_path = make_temp_json_path("gpui-devices")?;
    let status = Command::new("xcrun")
        .args(["devicectl", "list", "devices", "--json-output"])
        .arg(&output_path)
        .status()
        .context("failed to run devicectl list devices")?;
    if !status.success() {
        let _ = fs::remove_file(&output_path);
        bail!("devicectl list devices failed");
    }

    let contents = fs::read_to_string(&output_path)
        .with_context(|| format!("failed to read {}", output_path.display()))?;
    let _ = fs::remove_file(&output_path);
    let json: Value = serde_json::from_str(&contents).context("failed to parse devicectl JSON")?;
    let devices = json
        .get("result")
        .and_then(|value| value.get("devices"))
        .and_then(Value::as_array)
        .context("devicectl output missing devices array")?;

    let mut out = Vec::new();
    for device in devices {
        let hardware = device
            .get("hardwareProperties")
            .and_then(Value::as_object)
            .context("device missing hardwareProperties")?;
        if hardware.get("reality").and_then(Value::as_str) != Some("physical") {
            continue;
        }
        if hardware.get("platform").and_then(Value::as_str) != Some("iOS") {
            continue;
        }

        let core_id = device
            .get("identifier")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let legacy_udid = hardware
            .get("udid")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if core_id.is_empty() || legacy_udid.is_empty() {
            continue;
        }

        let connection = device
            .get("connectionProperties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let tunnel_state = connection
            .get("tunnelState")
            .and_then(Value::as_str)
            .unwrap_or("unavailable");
        let status = match tunnel_state {
            "unavailable" => DeviceStatus::Unavailable,
            "disconnected" => DeviceStatus::Reachable,
            _ => DeviceStatus::Available,
        };
        let properties = device
            .get("deviceProperties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        out.push(Device {
            core_id,
            legacy_udid,
            name: properties
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            model: hardware
                .get("marketingName")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            os_version: properties
                .get("osVersionNumber")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string(),
            status,
        });
    }
    Ok(out)
}

fn select_device(requested: Option<&str>) -> Result<Device> {
    let devices = fetch_physical_devices()?;
    if devices.is_empty() {
        bail!(
            "no available physical iOS device found; run `cargo gpui devices ios` to inspect connectivity"
        );
    }

    if let Some(requested_id) = requested {
        let requested = requested_id.trim();
        let device = devices
            .into_iter()
            .find(|device| {
                device.core_id == requested
                    || device.legacy_udid == requested
                    || device.name == requested
            })
            .with_context(|| format!("device `{requested}` not found"))?;

        if matches!(device.status, DeviceStatus::Unavailable) {
            bail!(
                "device `{}` is unavailable; unlock it, trust this Mac, enable Developer Mode, and connect it by USB or usable Wi-Fi pairing",
                device.name
            );
        }

        return Ok(device);
    }

    devices
        .into_iter()
        .find(|device| matches!(device.status, DeviceStatus::Available | DeviceStatus::Reachable))
        .context("no usable physical iOS device found; unlock it, trust this Mac, enable Developer Mode, and connect it by USB or usable Wi-Fi pairing")
}

fn install_on_device(device: &Device, app_path: &Path) -> Result<()> {
    println!("Installing on {}...", device.name);
    let status = Command::new("xcrun")
        .args([
            "devicectl",
            "device",
            "install",
            "app",
            "--device",
            &device.core_id,
        ])
        .arg(app_path)
        .status()
        .context("failed to install app on device")?;
    ensure_success(status, "device install failed")
}

fn spawn_log_listener(port: u16) -> Result<ChildGuard> {
    clear_stale_listener(port)?;

    let child = if which("socat").is_ok() {
        Command::new("socat")
            .args(["-u", &format!("TCP-LISTEN:{port},reuseaddr,fork"), "STDOUT"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("failed to start socat log listener")?
    } else {
        Command::new("nc")
            .args(["-l", "-k", &port.to_string()])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("failed to start netcat log listener")?
    };

    println!("Log listener on port {port} (PID {})", child.id());
    Ok(ChildGuard { child: Some(child) })
}

fn clear_stale_listener(port: u16) -> Result<()> {
    if which("lsof").is_err() {
        return Ok(());
    }
    let output = Command::new("lsof")
        .arg(format!("-tiTCP:{port}"))
        .arg("-sTCP:LISTEN")
        .output()
        .context("failed to run lsof")?;
    if !output.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(pid) = stdout.lines().next() {
        let status = Command::new("kill")
            .arg(pid)
            .status()
            .with_context(|| format!("failed to kill stale listener process {pid}"))?;
        ensure_success(status, "failed to kill stale log listener")?;
    }
    Ok(())
}

fn rustup_which(rustup: &Path, tool: &str) -> Result<PathBuf> {
    let output = Command::new(rustup)
        .args(["which", tool])
        .output()
        .with_context(|| format!("failed to query rustup for {tool}"))?;
    if !output.status.success() {
        bail!("rustup could not resolve `{tool}`");
    }
    let path = String::from_utf8(output.stdout).context("rustup returned non-utf8 path")?;
    Ok(PathBuf::from(path.trim()))
}

fn detect_host_ip() -> Option<String> {
    ["en0", "en1", "en2", "en3", "en4"]
        .into_iter()
        .find_map(|iface| {
            let output = Command::new("ipconfig")
                .args(["getifaddr", iface])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if ip.is_empty() { None } else { Some(ip) }
        })
}

fn host_architecture() -> Result<String> {
    let output = Command::new("uname")
        .arg("-m")
        .output()
        .context("failed to query host architecture with uname")?;
    if !output.status.success() {
        bail!("uname -m failed");
    }
    Ok(String::from_utf8(output.stdout)
        .context("uname returned non-utf8 output")?
        .trim()
        .to_string())
}

fn build_target(
    host: &IosHost,
    rustup: &Path,
    cargo: &Path,
    rustc: &Path,
    target: &str,
    release: bool,
    log_relay: Option<&str>,
) -> Result<()> {
    let _ = Command::new(rustup)
        .args(["target", "add", target])
        .status();

    let mut command = Command::new(cargo);
    command
        .current_dir(&host.paths.workspace_root)
        .env("RUSTC", rustc)
        .arg("build")
        .arg("-p")
        .arg("gpui_ios")
        .arg("--target")
        .arg(target);
    if release {
        command.arg("--release");
    }
    if let Some(log_relay) = log_relay {
        command.env("GPUI_LOG_RELAY", log_relay);
    }

    let status = command
        .status()
        .with_context(|| format!("failed to build Rust static library for target {target}"))?;
    ensure_success(status, &format!("Rust build failed for target {target}"))
}

fn copy_static_lib(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_file() {
        bail!("missing Rust static library at {}", source.display());
    }
    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to copy Rust static library from {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn command_json(command: &mut Command, context: &str) -> Result<Value> {
    let output = command.output().with_context(|| context.to_string())?;
    if !output.status.success() {
        bail!("{context}");
    }
    serde_json::from_slice(&output.stdout).with_context(|| context.to_string())
}

fn make_temp_json_path(prefix: &str) -> Result<PathBuf> {
    let mut path = env::temp_dir();
    let file_name = format!("{prefix}-{}.json", std::process::id());
    path.push(file_name);
    Ok(path)
}

trait DeviceStatusExt {
    fn as_str(&self) -> &'static str;
}

impl DeviceStatusExt for DeviceStatus {
    fn as_str(&self) -> &'static str {
        match self {
            DeviceStatus::Available => "available",
            DeviceStatus::Reachable => "reachable",
            DeviceStatus::Unavailable => "unavailable",
        }
    }
}
