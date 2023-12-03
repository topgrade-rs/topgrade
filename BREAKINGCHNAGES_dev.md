1. In 13.0.0, we introduced a new feature, pushing git repos, now this feature 
   has been removed as some users are not satisfied with it.

   For configuration entries, the following ones are gone:

   ```toml
   [git]
   pull_only_repos = []
   push_only_repos = []
   pull_arguments = ""
   push_arguments = ""
   ```