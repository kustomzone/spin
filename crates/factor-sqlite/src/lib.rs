mod host;
pub mod runtime_config;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use host::InstanceState;

use async_trait::async_trait;
use spin_factors::{anyhow, Factor, RuntimeFactors};
use spin_locked_app::MetadataKey;
use spin_world::v1::sqlite as v1;
use spin_world::v2::sqlite as v2;

pub struct SqliteFactor {
    runtime_config_resolver: Arc<dyn runtime_config::RuntimeConfigResolver>,
}

impl SqliteFactor {
    /// Create a new `SqliteFactor`
    pub fn new(
        runtime_config_resolver: impl runtime_config::RuntimeConfigResolver + 'static,
    ) -> Self {
        Self {
            runtime_config_resolver: Arc::new(runtime_config_resolver),
        }
    }
}

impl Factor for SqliteFactor {
    type RuntimeConfig = runtime_config::RuntimeConfig;
    type AppState = AppState;
    type InstanceBuilder = InstanceState;

    fn init<T: RuntimeFactors>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(v1::add_to_linker)?;
        ctx.link_bindings(v2::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: spin_factors::RuntimeFactors>(
        &self,
        mut ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        let mut connection_pools = HashMap::new();
        if let Some(runtime_config) = ctx.take_runtime_config() {
            for (
                database_label,
                runtime_config::StoreConfig {
                    type_: database_kind,
                    config,
                },
            ) in runtime_config.store_configs
            {
                let pool = self
                    .runtime_config_resolver
                    .get_pool(&database_kind, config)?;
                connection_pools.insert(database_label, pool);
            }
        }

        let allowed_databases = ctx
            .app()
            .components()
            .map(|component| {
                Ok((
                    component.id().to_string(),
                    component
                        .get_metadata(ALLOWED_DATABASES_KEY)?
                        .unwrap_or_default()
                        .into_iter()
                        .collect::<HashSet<_>>()
                        .into(),
                ))
            })
            .collect::<anyhow::Result<_>>()?;
        let resolver = self.runtime_config_resolver.clone();
        Ok(AppState {
            allowed_databases,
            get_connection_pool: Arc::new(move |label| {
                connection_pools
                    .get(label)
                    .cloned()
                    .or_else(|| resolver.default(label))
            }),
        })
    }

    fn prepare<T: spin_factors::RuntimeFactors>(
        &self,
        ctx: spin_factors::PrepareContext<Self>,
        _builders: &mut spin_factors::InstanceBuilders<T>,
    ) -> spin_factors::anyhow::Result<Self::InstanceBuilder> {
        let allowed_databases = ctx
            .app_state()
            .allowed_databases
            .get(ctx.app_component().id())
            .cloned()
            .unwrap_or_default();
        let get_connection_pool = ctx.app_state().get_connection_pool.clone();
        Ok(InstanceState::new(allowed_databases, get_connection_pool))
    }
}

pub const ALLOWED_DATABASES_KEY: MetadataKey<Vec<String>> = MetadataKey::new("databases");

pub struct AppState {
    /// A map from component id to a set of allowed database labels.
    allowed_databases: HashMap<String, Arc<HashSet<String>>>,
    /// A function for mapping from database name to a connection pool
    get_connection_pool: host::ConnectionPoolGetter,
}

/// A pool of connections for a particular SQLite database
#[async_trait]
pub trait ConnectionPool: Send + Sync {
    /// Get a `Connection` from the pool
    async fn get_connection(&self) -> Result<Arc<dyn Connection + 'static>, v2::Error>;
}

/// A simple [`ConnectionPool`] that always creates a new connection.
pub struct SimpleConnectionPool(
    Box<dyn Fn() -> anyhow::Result<Arc<dyn Connection + 'static>> + Send + Sync>,
);

impl SimpleConnectionPool {
    /// Create a new `SimpleConnectionPool` with the given connection factory.
    pub fn new(
        factory: impl Fn() -> anyhow::Result<Arc<dyn Connection + 'static>> + Send + Sync + 'static,
    ) -> Self {
        Self(Box::new(factory))
    }
}

#[async_trait::async_trait]
impl ConnectionPool for SimpleConnectionPool {
    async fn get_connection(&self) -> Result<Arc<dyn Connection + 'static>, v2::Error> {
        (self.0)().map_err(|_| v2::Error::InvalidConnection)
    }
}

/// A trait abstracting over operations to a SQLite database
#[async_trait]
pub trait Connection: Send + Sync {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<v2::Value>,
    ) -> Result<v2::QueryResult, v2::Error>;

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()>;
}
