#[cfg(test)]
mod tests {
    use serial_test_derive::serial;

    #[test]
    #[serial(alpha)]
    fn test_serial_attribute() {
    }
}
