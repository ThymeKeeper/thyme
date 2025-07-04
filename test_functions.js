// Test function and method call highlighting

// Regular function calls
console.log("Hello world");
parseInt("123");
setTimeout(callback, 1000);

// Method calls
string.toUpperCase();
array.map(x => x * 2);
object.hasOwnProperty("key");
document.getElementById("test");

// Chained method calls
array
  .filter(x => x > 0)
  .map(x => x * 2)
  .reduce((a, b) => a + b);

// Functions with underscores
my_function();
_privateFunction();
__special_function__();

// Method calls with underscores
object.my_method();
instance._private_method();
