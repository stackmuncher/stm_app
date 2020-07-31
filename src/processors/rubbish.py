-- Uplifted from https://www.mssqltips.com/sqlservertip/3439/script-to-drop-all-orphaned-sql-server-database-users/

-- Create this SP in MASTER and then run `EXEC sp_Drop_OrphanedUsers` from every DB.
-- A unsafe alternative is `EXECUTE master.sys.sp_MSforeachdb 'USE [?];EXEC sp_Drop_OrphanedUsers'` from MASTER to apply to all DBs. 

use [master]
go
drop proc if exists dbo.sp_Drop_OrphanedUsers
go
create proc dbo.sp_Drop_OrphanedUsers
as
begin
 set nocount on
 -- get orphaned users  
 declare @user varchar(max) 
 declare c_orphaned_user cursor for 
  select name
  from sys.database_principals 
  where type in ('G','S','U') 
  and authentication_type<>2 -- Use this filter only if you are running on SQL Server 2012 and major versions and you have "contained databases"
  and [sid] not in ( select [sid] from sys.server_principals where type in ('G','S','U') ) 
  and name not in ('dbo','guest','INFORMATION_SCHEMA','sys','MS_DataCollectorInternalUser')  open c_orphaned_user 
 fetch next from c_orphaned_user into @user
 while(@@FETCH_STATUS=0)
 begin
  -- alter schemas for user 
  declare @schema_name varchar(max) 
  declare c_schema cursor for 
   select name from  sys.schemas where USER_NAME(principal_id)=@user
  open c_schema 
  fetch next from c_schema into @schema_name
  while (@@FETCH_STATUS=0)
  begin
   declare @sql_schema varchar(max)
   select @sql_schema='ALTER AUTHORIZATION ON SCHEMA::['+@schema_name+ '] TO [dbo]'
   --print @sql_schema
   exec(@sql_schema)
   fetch next from c_schema into @schema_name
  end
  close c_schema
  deallocate c_schema   
  
  -- alter roles for user 
  declare @dp_name varchar(max) 
  declare c_database_principal cursor for 
   select name from sys.database_principals
   where type='R' and user_name(owning_principal_id)=@user
  open c_database_principal
  fetch next from c_database_principal into @dp_name
  while (@@FETCH_STATUS=0)
  begin
   declare @sql_database_principal  varchar(max)
   select @sql_database_principal ='ALTER AUTHORIZATION ON ROLE::['+@dp_name+ '] TO [dbo]'
   --print @sql_database_principal 
   exec(@sql_database_principal )
   fetch next from c_database_principal into @dp_name
  end
  close c_database_principal
  deallocate c_database_principal
    
  -- drop roles for user 
  declare @role_name varchar(max) 
  declare c_role cursor for 
   select dp.name--,USER_NAME(member_principal_id)
   from sys.database_role_members drm
   inner join sys.database_principals dp 
   on dp.principal_id= drm.role_principal_id
   where USER_NAME(member_principal_id)=@user 
  open c_role 
  fetch next from c_role into @role_name
  while (@@FETCH_STATUS=0)
  begin
   declare @sql_role varchar(max)
   select @sql_role='EXEC sp_droprolemember N'''+@role_name+''', N'''+@user+''''
   --print @sql_role
   exec (@sql_role)
   fetch next from c_role into @role_name
  end
  close c_role
  deallocate c_role   
      
  -- drop user
  declare @sql_user varchar(max)
  set @sql_user='DROP USER ['+@user +']'
  --print @sql_user
  exec (@sql_user)
  fetch next from c_orphaned_user into @user
 end
 close c_orphaned_user
 deallocate c_orphaned_user
 set nocount off
end
go
-- mark stored procedure as a system stored procedure
exec sys.sp_MS_marksystemobject sp_Drop_OrphanedUsers
go