//! Native macOS notification through Apple's UserNotifications framework, so the alert shows the
//! PocketRemote icon (a notification sent through `osascript` always shows the script icon).
//!
//! Only used when running from the installed `.app` bundle (the framework needs a bundle identity);
//! development builds and any failure fall back to a plain `osascript` notification, so an alert is
//! never lost. The text is a fixed string: nothing from the session is ever shown or logged.

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_foundation::{NSError, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationRequest,
    UNNotificationSound, UNUserNotificationCenter,
};

/// True when this process is the installed bundle (the framework crashes without a bundle identity).
pub fn in_bundle() -> bool {
    std::env::current_exe()
        .ok()
        .is_some_and(|p| p.to_string_lossy().contains(".app/Contents/MacOS/"))
}

fn plain(message: &'static str) {
    let script = format!("display notification \"{message}\" with title \"PocketRemote\"");
    let _ = std::process::Command::new("osascript")
        .args(["-e", &script])
        .status();
}

/// Posts the notification. Asks for permission first (macOS shows its prompt once); if it is not
/// granted, or anything fails, sends the plain fallback instead.
pub fn send(message: &'static str) {
    if !in_bundle() {
        std::thread::spawn(move || plain(message));
        return;
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let on_auth = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
        if granted.as_bool() {
            post(message);
        } else {
            plain(message);
        }
    });
    center.requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
        &on_auth,
    );
}

fn post(message: &'static str) {
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str("PocketRemote"));
    content.setBody(&NSString::from_str(message));
    content.setSound(Some(&UNNotificationSound::defaultSound()));
    let id = NSString::from_str("pocketremote.phone-connected");
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(&id, &content, None);
    let on_done = RcBlock::new(move |error: *mut NSError| {
        if !error.is_null() {
            plain(message);
        }
    });
    center.addNotificationRequest_withCompletionHandler(&request, Some(&on_done));
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_development_or_test_binary_is_not_the_installed_bundle() {
        // The native framework needs a bundle identity; outside the .app we use the plain fallback.
        assert!(!super::in_bundle());
    }
}
