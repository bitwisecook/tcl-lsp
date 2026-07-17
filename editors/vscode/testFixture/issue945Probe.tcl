proc greet {} { return hello }
namespace which -command greet
namespace which -command no_such_command_xyz
