//! Authentication commands

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::commands::CommandContext;
use crate::interactive::{confirm, prompt_input, prompt_password};
use crate::output::colors;

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    user: UserInfo,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    id: String,
    email: String,
    username: String,
}

/// Login to the LLM Benchmark Exchange
pub async fn login(ctx: &mut CommandContext, token: Option<String>) -> Result<()> {
    let auth_token = if let Some(t) = token {
        // Token-based login
        println!("{}", colors::info("Logging in with provided token..."));
        t
    } else {
        // Interactive login
        println!("{}", colors::bold("Login to LLM Benchmark Exchange"));
        println!();

        let email = prompt_input("Email")?;
        let password = prompt_password("Password")?;

        println!();
        println!("{}", colors::info("Authenticating..."));

        let request = LoginRequest { email, password };

        let response: LoginResponse = ctx
            .client
            .post("/api/v1/auth/login", &request)
            .await
            .map_err(|e| anyhow::anyhow!("Login failed: {}", e))?;

        println!(
            "{}",
            colors::success(&format!("Welcome, {}!", response.user.username))
        );

        response.token
    };

    // Save token to config
    ctx.config.set_auth_token(auth_token)?;

    // Update client with new token
    ctx.client = crate::client::ApiClient::from_config(&ctx.config)?;

    println!("{}", colors::success("Successfully logged in!"));

    Ok(())
}

/// Logout from the LLM Benchmark Exchange
pub async fn logout(ctx: &mut CommandContext) -> Result<()> {
    if !ctx.config.is_authenticated() {
        println!("{}", colors::warning("Not currently logged in."));
        return Ok(());
    }

    let confirmed = confirm("Are you sure you want to logout?")?;

    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    ctx.config.clear_auth_token()?;

    // Update client without token
    ctx.client = crate::client::ApiClient::from_config(&ctx.config)?;

    println!("{}", colors::success("Successfully logged out!"));

    Ok(())
}

/// Show current user information
pub async fn whoami(ctx: &CommandContext) -> Result<()> {
    ctx.require_auth()?;

    println!("{}", colors::info("Fetching user information..."));

    let user: UserInfo = ctx
        .client
        .get("/api/v1/auth/me")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user info: {}", e))?;

    println!();
    println!("{}", colors::bold("Current User:"));
    println!("  ID:       {}", user.id);
    println!("  Username: {}", user.username);
    println!("  Email:    {}", user.email);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_serialization() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test@example.com"));
    }
}
