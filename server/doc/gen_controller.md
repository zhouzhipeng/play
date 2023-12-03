## how to use [controller_template.txt](controller_template.txt)

> use `codebox` tool  > `String Magic`

1. copy model_template.txt content into input area.
2. run and copy result.

## principle
* should always write a test file in `tests` folder for your controller, like [index_controller_test.rs](..%2F..%2F..%2Ftests%2Findex_controller_test.rs)
* the structure of controller should be the same, remember to register your `init` method in `mod.rs`
* `get` --> return html content
* `post` ---> return json data