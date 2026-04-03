use validator::ValidateEmail;
    
#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        if ValidateEmail::validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests{
    use super::SubscriberEmail;
    use claim::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
    
    use quickcheck::{Arbitrary, Gen};
    
    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);
    
    impl Arbitrary for ValidEmailFixture {
        fn arbitrary(_g: &mut Gen) -> Self {
            // Just generate a random safe email, no need to use g
            let email: String = SafeEmail().fake();
            Self(email)
        }
    }



    use quickcheck_macros::quickcheck;
    
    #[quickcheck]
    fn valid_emails_are_parsed_successfully(email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(email.0).is_ok()
    }

}