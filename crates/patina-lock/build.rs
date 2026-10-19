fn main() {
    // PAM, for checking the password (libpam0g-dev on Debian).
    println!("cargo:rustc-link-lib=pam");
}
