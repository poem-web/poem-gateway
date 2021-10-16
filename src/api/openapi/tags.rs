use poem_openapi::Tags;

#[derive(Tags)]
pub enum ApiTags {
    /// Listener operation
    Listener,

    /// Consumer operation
    Consumer,

    /// Route operation
    Route,

    /// Service operation
    Service,

    /// Global plugin operation
    GlobalPlugin,
}
