#[cfg(test)]
mod compiler_integration_tests {
    use vue_compiler_core::{
        BaseCompileSource, CodegenResult, CompilerOptions, base_compile as compile,
    };

    const SOURCE: &'static str = r#"
  <div id="foo" :class="bar.baz">
    {{ world.burn() }}
    <div v-if="ok">yes</div>
    <template v-else>no</template>
    <div v-for="(value, index) in list"><span>{{ value + index }}</span></div>
  </div>
  "#;
    #[test]
    fn function_mode() {
        let options = CompilerOptions::default();

        let CodegenResult { code, ast, .. } = compile(
            BaseCompileSource::String(SOURCE.trim().to_string()),
            options,
        );

        // println!("{ast:#?}");
        println!("{code}");
    }
}
