# Versioned Contract example

Shows basic example of how you can setup your contract to be versioned using state as an enum.

This can be a useful setup if you expect your contract to be upgradable and do not want to write migration functions or manually attempt to deserialize old state formats (which can be error prone).
