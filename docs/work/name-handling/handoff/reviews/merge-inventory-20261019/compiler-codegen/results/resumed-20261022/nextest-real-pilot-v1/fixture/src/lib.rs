#[cfg(test)]
mod tests {
    #[test]
    fn actual_reporter_pass() {
        println!("capture pilot passing stdout");
        eprintln!("capture pilot passing stderr");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn actual_reporter_failure() {
        println!("capture pilot failing stdout");
        eprintln!("capture pilot failing stderr");
        panic!("intentional tool-pilot failure");
    }
}
