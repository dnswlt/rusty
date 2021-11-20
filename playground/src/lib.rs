#[cfg(test)]
mod tests {
    #[test]
    fn into_iter_yields_moved_value() {
        let xs = vec![String::from("foosen")];
        let xs_bytes: usize = xs.into_iter().map(|x: String| x.len()).sum();
        assert_eq!(xs_bytes, 6);
    }
    #[test]
    fn into_iter_yields_moved_value_ref_case() {
        let xs = vec![String::from("foosen")];
        let ys: Vec<&str> = xs.iter().map(|s| &s[..]).collect();
        let ys_bytes: usize = ys.into_iter().map(|x: &str| x.len()).sum();
        assert_eq!(ys_bytes, 6);
    }

    #[test]
    fn into_iter_for_ref_to_array_of_str() {
        let xs: [&str; 1] = ["foosen"];
        let ys = &xs;
        let xs_bytes: usize = xs.into_iter().map(|x: &str| x.len()).sum();
        let ys_bytes: usize = ys.into_iter().map(|x: &&str| x.len()).sum();
        assert_eq!(xs_bytes, 6);
        assert_eq!(ys_bytes, 6);
    }
    #[test]
    fn into_iter_for_ref_to_array_of_string() {
        // If we into_iter over a borrowed array, even into_iter yields references.
        // It seems to behave exactly like iter!
        let xs: [String;1] = [String::from("foosen")];
        let ys = &xs;
        let ys_bytes: usize = ys.into_iter().map(|x: &String| x.len()).sum();
        let ys_bytes2: usize = ys.iter().map(|x: &String| x.len()).sum();
        assert_eq!(ys_bytes, 6);
        assert_eq!(ys_bytes2, 6);
    }
}
