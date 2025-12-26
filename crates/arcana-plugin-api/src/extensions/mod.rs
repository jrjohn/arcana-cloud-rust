//! Plugin extension point definitions.

use serde::{Deserialize, Serialize};

/// REST endpoint extension for plugins to register HTTP routes.
pub trait RestEndpointExtension: Send + Sync {
    /// Returns the routes provided by this extension.
    fn routes(&self) -> Vec<RouteDefinition>;

    /// Handles an HTTP request.
    fn handle_request(&self, request: HttpRequest) -> HttpResponse;
}

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Route definition for REST endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDefinition {
    pub method: HttpMethod,
    pub path: String,
    pub handler_name: String,
    pub requires_auth: bool,
    pub required_permission: Option<String>,
}

/// HTTP request from the platform to a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub query_params: Vec<(String, String)>,
    pub path_params: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub user_id: Option<String>,
}

/// HTTP response from a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

/// Service extension for plugins to register business logic components.
pub trait ServiceExtension: Send + Sync {
    /// Returns information about the service.
    fn info(&self) -> ServiceInfo;

    /// Invokes the service with the given method and parameters.
    fn invoke(&self, method: &str, params: &str) -> Result<String, String>;
}

/// Service information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub key: String,
    pub interface_name: String,
    pub description: String,
    pub priority: i32,
}

/// Event listener extension for plugins to handle platform events.
pub trait EventListenerExtension: Send + Sync {
    /// Returns the event subscriptions.
    fn subscriptions(&self) -> EventSubscription;

    /// Handles an event.
    fn handle_event(&self, event: PluginEvent) -> Result<(), String>;
}

/// Event subscription configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    pub event_types: Vec<String>,
    pub order: i32,
    pub async_handling: bool,
}

/// Event from the platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    pub event_type: String,
    pub timestamp: u64,
    pub payload: String,
    pub source_plugin: Option<String>,
}

/// Scheduled job extension for plugins to run background tasks.
pub trait ScheduledJobExtension: Send + Sync {
    /// Returns the job configuration.
    fn config(&self) -> JobConfig;

    /// Executes the job.
    fn execute(&self, ctx: JobContext) -> Result<(), String>;

    /// Checks if cancellation was requested.
    fn is_cancelled(&self) -> bool;
}

/// Schedule type for jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Schedule {
    /// Cron expression.
    Cron(String),
    /// Fixed delay in milliseconds.
    FixedDelay(u64),
    /// Fixed rate in milliseconds.
    FixedRate(u64),
}

/// Job configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub key: String,
    pub schedule: Schedule,
    pub description: String,
    pub enabled: bool,
    pub allow_concurrent: bool,
}

/// Job execution context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobContext {
    pub job_key: String,
    pub scheduled_time: u64,
    pub execution_id: String,
}

/// SSR view extension for server-side rendering.
pub trait SsrViewExtension: Send + Sync {
    /// Returns the view configuration.
    fn config(&self) -> SsrViewConfig;

    /// Returns initial props for SSR.
    fn get_initial_props(&self, ctx: SsrContext) -> Result<String, String>;
}

/// SSR framework type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SsrFramework {
    React,
    Angular,
}

/// SSR view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsrViewConfig {
    pub key: String,
    pub path: String,
    pub framework: SsrFramework,
    pub entry: String,
    pub title: String,
    pub requires_permission: Option<String>,
    pub cache_duration: u32,
}

/// SSR rendering context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsrContext {
    pub path: String,
    pub query_params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub user_id: Option<String>,
    pub locale: String,
}
