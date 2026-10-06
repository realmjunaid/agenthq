/// (sql, target user_version) pairs, applied in order.
pub const MIGRATIONS: &[(&str, u32)] = &[
    (include_str!("../../migrations/001_agents.sql"), 1),
    (include_str!("../../migrations/002_activity.sql"), 2),
    (include_str!("../../migrations/003_catalog.sql"), 3),
    (include_str!("../../migrations/004_events.sql"), 4),
    (include_str!("../../migrations/005_mcp_details.sql"), 5),
    (include_str!("../../migrations/006_projects_relax.sql"), 6),
];
