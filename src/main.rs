use glob::glob;
use regex::Regex;
use reqwest::Url;
use std::env::consts::ARCH;
use std::env;
use std::fs::{remove_file, File, create_dir_all};
use std::io::copy;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let env_args: Vec<String> = env::args().collect();
    let default_cmd = "sudo".to_string();
    let cmd = env_args.get(1).unwrap_or(&default_cmd);

    // Remove existing RPM files

	let rpm_downloads = "downloads.rpm";
	let rpm_glob = format!("{}/*.rpm", rpm_downloads);

	create_dir_all(rpm_downloads)?;	

    for entry in glob(&rpm_glob)? {
        if let Ok(path) = entry {
            println!("Removing {:?}", path);
            remove_file(path)?;
        }
    }


    let base_url = Url::parse("https://dl.fedoraproject.org/pub/fedora/linux/development/rawhide/Everything/x86_64/os/Packages/q/")?;
    let body = reqwest::get(base_url.clone()).await?.text().await?;

    let re = Regex::new(&format!(r#"href="(qemu-user-static[^"]*{}[^"]*)""#, ARCH)).unwrap();

    let mut rpm_files = Vec::new();

    for cap in re.captures_iter(&body) {
        let href = &cap[1];
        let url = base_url.join(href)?;
        let response = reqwest::get(url.clone()).await?;
        let fname = url
            .path_segments()
            .and_then(std::iter::Iterator::last)
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or("tmp.bin")
            .to_string(); // Clone the string here
        let dest_file = format!("{}/{}", rpm_downloads, fname);
        let mut dest = File::create(&dest_file)?;
        let content = response.bytes().await?;
        copy(&mut content.as_ref(), &mut dest)?;
        println!("Downloading {}", fname);
        rpm_files.push(dest_file);
    }

    let argv: Vec<String> = vec!["rpm".into(), "-Uvh".into()]
        .into_iter()
        .chain(rpm_files.into_iter())
        .collect();

    let mut child = Command::new(&cmd)
        .args(&argv)
        .spawn()
        .map_err(|e| e.to_string())?;
    let exit_status = child.wait();
    let rc = exit_status.map_err(|e| e.to_string())?.code().unwrap_or(1);

	if ![0, 41].contains(&rc) {
        Err(format!("{:?}\nfailed with exit status: {}", &argv, rc).into())
    } else {
        Ok(())
    }
}
