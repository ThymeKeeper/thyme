// Test all syntax highlighting

// Keywords
if (true) {
    const x = 10;
    let y = 20;
}

// Strings
const str1 = "Hello World";
const str2 = 'Single quotes';

// Comments
// This is a comment
/* Multi-line
   comment */

// Numbers
const num1 = 42;
const num2 = 3.14;
const hex = 0xFF;

// Regular function calls - should be highlighted
console.log("Testing function highlight");
parseInt("123");
setTimeout(callback, 1000);
Math.random();
Array.isArray([]);

// Method calls - should be highlighted
"hello".toUpperCase();
[1, 2, 3].map(x => x * 2);
object.hasOwnProperty("key");
document.getElementById("test");

// Function definitions - should be highlighted
function myFunction() {
    return true;
}

// Not functions (no parentheses) - should NOT be highlighted
const notAFunction = console;
const property = object.property;
const value = array.length;

// Identifiers and types
const MyClass = class {};
const CONSTANT_VALUE = 100;

// Operators
const sum = 1 + 2;
const condition = x > 0 && y < 10;
