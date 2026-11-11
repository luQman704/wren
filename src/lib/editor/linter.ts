import { linter, type Diagnostic } from '@codemirror/lint';
import { syntaxTree } from '@codemirror/language';

export function syntaxErrorLinter() {
  return linter((view) => {
    const diagnostics: Diagnostic[] = [];
    syntaxTree(view.state)
      .cursor()
      .iterate((node) => {
        if (node.type.isError) {
          diagnostics.push({
            from: node.from,
            to: Math.max(node.to, node.from + 1),
            severity: 'error',
            message: 'Syntax error',
          });
        }
      });
    return diagnostics;
  });
}
