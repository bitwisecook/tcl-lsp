proc greet {name} {
    puts "Hello [string totitle $name]!"
    #                          ^--- select [string totitle $name]
}
