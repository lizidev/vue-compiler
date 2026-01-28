#[cfg(test)]
mod compiler_integration_tests {
    use insta::assert_snapshot;
    use vue_compiler_core::{
        BaseCompileSource, CodegenMode, CodegenResult, CompilerOptions, base_compile as compile,
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
        let mut options = CompilerOptions::default();
        options.filename = Some("foo.vue".to_string());

        let CodegenResult { code, .. } = compile(
            BaseCompileSource::String(SOURCE.trim().to_string()),
            options,
        );

        assert_snapshot!(code);
    }

    #[test]
    fn module_mode() {
        let mut options = CompilerOptions::default();
        options.mode = Some(CodegenMode::Module);
        options.filename = Some("foo.vue".to_string());

        let CodegenResult { code, .. } = compile(
            BaseCompileSource::String(SOURCE.trim().to_string()),
            options,
        );

        assert_snapshot!(code);
    }
}
