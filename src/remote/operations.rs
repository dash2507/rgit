use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path;

use remote::GitClient;
use remote::httpclient::GitHttpClient;
use packfile::PackFile;
use packfile::refs;
use packfile::refs::GitRef;
use store;

pub fn clone_priv(repo: &str, dir: &str) -> io::Result<()> {
    println!("Cloning into \"{}\"...", dir);

    let mut client = GitHttpClient::new(repo);
    let refs = try!(client.discover_refs());
    let packfile_data = try!(client.fetch_packfile(&refs));

    let mut p = path::PathBuf::new();
    p.push(dir);
    p.push(".git");

    try!(fs::create_dir_all(&p));

    let mut filepath = p.clone();
    filepath.push("pack");

    let mut file = try!(File::create(&filepath));
    try!(file.write_all(&packfile_data[..]));
    let parsed_packfile = PackFile::parse(&packfile_data[..]);
    parsed_packfile.unpack_all(dir).expect("Error unpacking parsed packfile");

    try!(refs::create_refs(dir, &refs));
    try!(refs::update_head(dir, &refs));

    // Checkout head and format refs
    try!(store::checkout_head(dir));
    Ok(())
}

///
/// Lists remote refs available in the given repo.
///
pub fn ls_remote(host: &str, port: u16, repo: &str) -> i32 {
    //let mut client = GitTcpClient::connect(repo, host, port).unwrap();
    let mut client = GitHttpClient::new(repo);
    match client.discover_refs() {
        Ok(pktlines) => {
            print_packetlines(&pktlines);
            0
        },
        Err(_) => -1
    }
}

fn print_packetlines(pktlines: &[GitRef]) {
    for p in pktlines.iter() {
        let &GitRef{ref id, ref name} = p;
        print!("{}\t{}\n", id, name);
    }
}
