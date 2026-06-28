pub mod admin;
pub mod deposit;
pub mod initialize_pool;
pub mod swap;
pub mod withdraw;

// Deliberately NOT flattened with `pub use module::*`. Several modules
// each define their own `handler` function (initialize_pool, deposit,
// swap, withdraw) plus admin.rs defines two differently-named handlers —
// flattening would either collide on the name `handler` or hide which
// module a given Accounts struct belongs to. lib.rs references each
// module's items via their full path (e.g. `initialize_pool::InitializePool`,
// `initialize_pool::handler`) instead.
