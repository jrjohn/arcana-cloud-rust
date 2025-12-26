//! Core traits for Clean Architecture.

use crate::{ArcanaResult, Page, PageRequest};
use async_trait::async_trait;

/// Base repository trait for CRUD operations.
///
/// This trait defines the standard operations that all repositories
/// must implement, following the Repository pattern.
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    /// Finds an entity by its ID.
    async fn find_by_id(&self, id: &ID) -> ArcanaResult<Option<T>>;

    /// Finds all entities with pagination.
    async fn find_all(&self, page: PageRequest) -> ArcanaResult<Page<T>>;

    /// Saves a new entity.
    async fn save(&self, entity: &T) -> ArcanaResult<T>;

    /// Updates an existing entity.
    async fn update(&self, entity: &T) -> ArcanaResult<T>;

    /// Deletes an entity by its ID.
    async fn delete(&self, id: &ID) -> ArcanaResult<bool>;

    /// Checks if an entity exists by its ID.
    async fn exists(&self, id: &ID) -> ArcanaResult<bool>;

    /// Counts all entities.
    async fn count(&self) -> ArcanaResult<u64>;
}

/// Marker trait for service layer components.
///
/// Services contain business logic and orchestrate operations
/// across multiple repositories and external services.
pub trait Service: Send + Sync {}

/// Trait for domain events.
///
/// Domain events represent something significant that happened
/// in the domain and can be used for event-driven architectures.
pub trait DomainEvent: Send + Sync {
    /// Returns the event type name.
    fn event_type(&self) -> &'static str;

    /// Returns the aggregate ID that this event belongs to.
    fn aggregate_id(&self) -> String;

    /// Returns the event timestamp.
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc>;

    /// Serializes the event to JSON.
    fn to_json(&self) -> ArcanaResult<String>;
}

/// Trait for entities with a unique identifier.
pub trait Entity<ID> {
    /// Returns the entity's unique identifier.
    fn id(&self) -> &ID;
}

/// Trait for aggregate roots.
///
/// An aggregate root is the entry point to an aggregate,
/// which is a cluster of domain objects treated as a single unit.
pub trait AggregateRoot<ID>: Entity<ID> {
    /// Returns the domain events that have occurred on this aggregate.
    fn domain_events(&self) -> &[Box<dyn DomainEvent>];

    /// Clears all domain events.
    fn clear_domain_events(&mut self);
}

/// Trait for mapping between domain entities and DTOs.
pub trait Mapper<From, To> {
    /// Maps from source type to target type.
    fn map(from: From) -> To;
}

/// Trait for bidirectional mapping between domain entities and DTOs.
pub trait BiMapper<A, B>: Mapper<A, B> {
    /// Maps from target type back to source type.
    fn map_back(from: B) -> A;
}

/// Trait for use cases in the application layer.
///
/// Use cases represent a single action that can be performed
/// in the system, following the Command pattern.
#[async_trait]
pub trait UseCase<Request, Response>: Send + Sync
where
    Request: Send,
    Response: Send,
{
    /// Executes the use case.
    async fn execute(&self, request: Request) -> ArcanaResult<Response>;
}

/// Trait for query handlers in CQRS pattern.
#[async_trait]
pub trait QueryHandler<Query, Response>: Send + Sync
where
    Query: Send,
    Response: Send,
{
    /// Handles the query.
    async fn handle(&self, query: Query) -> ArcanaResult<Response>;
}

/// Trait for command handlers in CQRS pattern.
#[async_trait]
pub trait CommandHandler<Command, Response>: Send + Sync
where
    Command: Send,
    Response: Send,
{
    /// Handles the command.
    async fn handle(&self, command: Command) -> ArcanaResult<Response>;
}

/// Trait for event handlers.
#[async_trait]
pub trait EventHandler<E: DomainEvent>: Send + Sync {
    /// Handles the event.
    async fn handle(&self, event: &E) -> ArcanaResult<()>;
}

/// Trait for event publishers.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes a domain event.
    async fn publish(&self, event: Box<dyn DomainEvent>) -> ArcanaResult<()>;

    /// Publishes multiple domain events.
    async fn publish_all(&self, events: Vec<Box<dyn DomainEvent>>) -> ArcanaResult<()>;
}

/// Trait for health checks.
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Returns the name of this health check.
    fn name(&self) -> &str;

    /// Performs the health check.
    async fn check(&self) -> HealthStatus;
}

/// Health check status.
#[derive(Debug, Clone)]
pub enum HealthStatus {
    /// The component is healthy.
    Healthy,
    /// The component is degraded but functional.
    Degraded(String),
    /// The component is unhealthy.
    Unhealthy(String),
}

impl HealthStatus {
    /// Returns true if the status is healthy.
    #[must_use]
    pub const fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Returns true if the status is unhealthy.
    #[must_use]
    pub const fn is_unhealthy(&self) -> bool {
        matches!(self, Self::Unhealthy(_))
    }
}
