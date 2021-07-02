use ics_dm_cli::file;

#[test]

fn check_file_exists() {
    assert_eq!(Ok, file::file_exists());
}