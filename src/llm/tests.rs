use super::LlmError;

#[test]
fn classifies_client_errors_as_not_retryable() {
    let error = LlmError::from_http(401, "invalid API key".into());

    assert!(matches!(
        &error,
        LlmError::InvalidRequest { status: 401, body } if body.as_str() == "invalid API key"
    ));
}

#[test]
fn classifies_timeout_and_rate_limit_as_retryable() {
    for status in [408, 429] {
        let error = LlmError::from_http(status, "try again".into());

        assert!(matches!(
            &error,
            LlmError::Server { status: actual_status, body }
                if *actual_status == status && body.as_str() == "try again"
        ));
        assert!(error.is_retryable());
    }
}

#[test]
fn classifies_server_errors_as_retryable() {
    let error = LlmError::from_http(503, "service unavailable".into());

    assert!(matches!(
        &error,
        LlmError::Server { status: 503, body } if body.as_str() == "service unavailable"
    ));
    assert!(error.is_retryable());
}

#[test]
fn classifies_other_http_statuses() {
    let error = LlmError::from_http(302, "redirect".into());

    assert!(matches!(&error, LlmError::Other(message) if message.as_str() == "HTTP 302: redirect"));
    assert!(!error.is_retryable());
}
