# Migration Deployment Checklist

Use this checklist when deploying database migrations to any environment.

## Pre-Deployment

### Development Environment

- [ ] Review all migration files for syntax errors
- [ ] Ensure migrations are numbered sequentially (00001-00010)
- [ ] Verify all enum types are defined
- [ ] Check that all foreign key relationships are correct
- [ ] Review index coverage for common queries
- [ ] Validate constraint definitions
- [ ] Test migration runner script locally
- [ ] Run schema validation script
- [ ] Review generated documentation

### Testing

- [ ] Create test database
- [ ] Run all migrations on test database
- [ ] Verify all tables created
- [ ] Verify all indexes created
- [ ] Verify all materialized views created
- [ ] Verify all functions created
- [ ] Verify all triggers working
- [ ] Test soft delete functionality
- [ ] Test materialized view refresh
- [ ] Test partition creation
- [ ] Run performance tests on sample data
- [ ] Test rollback procedures (if applicable)

### Code Review

- [ ] Peer review of all migration files
- [ ] Verify migrations follow naming conventions
- [ ] Check for potential performance issues
- [ ] Validate security constraints
- [ ] Review audit logging coverage
- [ ] Ensure proper indexing strategy
- [ ] Verify partitioning configuration
- [ ] Check comments and documentation

## Deployment

### Pre-Deployment Checks

- [ ] Database backup completed
- [ ] Backup verified and restorable
- [ ] Migration window scheduled
- [ ] Stakeholders notified
- [ ] Rollback plan prepared
- [ ] Emergency contacts identified
- [ ] Monitoring alerts configured

### Staging Environment

- [ ] Deploy to staging environment
- [ ] Run all migrations
- [ ] Validate schema matches development
- [ ] Test application integration
- [ ] Run integration tests
- [ ] Load test with production-like data
- [ ] Monitor performance metrics
- [ ] Verify materialized views refresh
- [ ] Test backup/restore procedures
- [ ] Document any issues found

### Production Deployment

- [ ] Final backup before deployment
- [ ] Put application in maintenance mode (if needed)
- [ ] Run migration runner script
- [ ] Monitor migration progress
- [ ] Verify all migrations completed
- [ ] Run schema validation script
- [ ] Create initial partitions
- [ ] Refresh materialized views
- [ ] Test critical queries
- [ ] Verify application connectivity
- [ ] Run smoke tests
- [ ] Monitor error logs
- [ ] Check performance metrics
- [ ] Remove maintenance mode
- [ ] Notify stakeholders of completion

## Post-Deployment

### Immediate Checks (Within 1 Hour)

- [ ] Verify all services operational
- [ ] Check application error rates
- [ ] Monitor database connections
- [ ] Review slow query log
- [ ] Check materialized view freshness
- [ ] Verify backup jobs running
- [ ] Monitor disk space usage
- [ ] Review audit logs
- [ ] Test critical user workflows

### Short-term Monitoring (Day 1-7)

- [ ] Monitor query performance
- [ ] Track table growth rates
- [ ] Review partition usage
- [ ] Check index usage statistics
- [ ] Monitor materialized view refresh times
- [ ] Review database connection patterns
- [ ] Analyze slow queries
- [ ] Monitor cache hit ratios
- [ ] Review error patterns
- [ ] Collect user feedback

### Long-term Tasks (Week 1-4)

- [ ] Schedule regular materialized view refreshes
- [ ] Set up partition creation cron job
- [ ] Configure automated backups
- [ ] Set up monitoring dashboards
- [ ] Document operational procedures
- [ ] Plan for next migration cycle
- [ ] Review and optimize slow queries
- [ ] Update capacity planning
- [ ] Archive old partitions
- [ ] Security audit

## Environment-Specific Checklist

### Development

- [ ] Docker Compose configured
- [ ] Environment variables set
- [ ] Make commands tested
- [ ] SQLx offline mode configured
- [ ] pgAdmin access working

### Staging

- [ ] Connection pooling configured
- [ ] SSL/TLS enabled
- [ ] Backup schedule configured
- [ ] Monitoring configured
- [ ] Resource limits set
- [ ] Read replica configured (if needed)

### Production

- [ ] High availability configured
- [ ] Automated failover tested
- [ ] Backup encryption enabled
- [ ] Point-in-time recovery configured
- [ ] Connection pooling optimized
- [ ] Read replicas configured
- [ ] Monitoring and alerting active
- [ ] Security hardening applied
- [ ] Compliance requirements met
- [ ] Disaster recovery plan documented

## Troubleshooting Checklist

### Migration Fails

- [ ] Check error messages in logs
- [ ] Verify database connectivity
- [ ] Check user permissions
- [ ] Verify disk space available
- [ ] Check for lock conflicts
- [ ] Review previous migration state
- [ ] Validate SQL syntax
- [ ] Check dependency order

### Performance Issues

- [ ] Run VACUUM ANALYZE
- [ ] Check index usage
- [ ] Review query plans (EXPLAIN ANALYZE)
- [ ] Verify materialized views refreshed
- [ ] Check for missing indexes
- [ ] Review connection pool settings
- [ ] Monitor lock contention
- [ ] Check partition pruning

### Data Integrity Issues

- [ ] Verify foreign key constraints
- [ ] Check unique constraints
- [ ] Validate check constraints
- [ ] Review trigger logic
- [ ] Verify soft delete working
- [ ] Check audit log entries
- [ ] Validate enum values
- [ ] Review materialized view accuracy

## Rollback Checklist

### If Rollback Needed

- [ ] Stop application traffic
- [ ] Create current state backup
- [ ] Restore from pre-deployment backup
- [ ] Verify data integrity
- [ ] Test critical queries
- [ ] Restart application
- [ ] Monitor for errors
- [ ] Document rollback reason
- [ ] Notify stakeholders
- [ ] Plan corrective actions

## Documentation Updates

- [ ] Update schema documentation
- [ ] Document new tables/columns
- [ ] Update API documentation
- [ ] Update deployment procedures
- [ ] Document performance characteristics
- [ ] Update monitoring runbooks
- [ ] Update backup procedures
- [ ] Create release notes
- [ ] Update changelog

## Sign-off

### Development Team

- [ ] Database administrator approval
- [ ] Backend engineer review
- [ ] DevOps engineer review
- [ ] Security team review (if required)

### Stakeholders

- [ ] Product owner notified
- [ ] Operations team informed
- [ ] Support team briefed
- [ ] Management approval (for production)

## Completion

- [ ] All checklist items completed
- [ ] Documentation updated
- [ ] Team notified
- [ ] Lessons learned documented
- [ ] Next steps identified
- [ ] Success metrics tracked

---

**Deployment Date**: _________________

**Deployed By**: _________________

**Environment**: [ ] Development [ ] Staging [ ] Production

**Migration Version**: 00001 through 00010

**Status**: [ ] Success [ ] Partial [ ] Rolled Back

**Notes**:
_________________________________________________________________
_________________________________________________________________
_________________________________________________________________
