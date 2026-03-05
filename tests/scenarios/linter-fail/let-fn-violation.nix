# This file should trigger the let-fn rule
# Functions defined inside let-in blocks are flagged
{
  example =
    let
      # BAD: Function defined inside let
      helper = x: x + 1;
      anotherHelper = { a, b }: a + b;
    in
    helper 5;

  nested =
    let
      outer =
        let
          # BAD: Deeply nested function
          innerFn = y: y * 2;
        in
        innerFn;
    in
    outer 10;
}
