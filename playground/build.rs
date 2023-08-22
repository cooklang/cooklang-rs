fn get_version() -> String {
    if let Ok(repo) = git2::Repository::discover(".") {
        let mut options = git2::DescribeOptions::new();
        options.describe_tags().show_commit_oid_as_fallback(true);
        if let Ok(describe) = repo.describe(&options) {
            if let Ok(format) = describe.format(None) {
                return format;
            }
        }
    }

    panic!("Can't read version from git2")
}

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("version");
    std::fs::write(&dest_path, get_version()).unwrap();
}
