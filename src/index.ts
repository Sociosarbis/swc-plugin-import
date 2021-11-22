import { CallExpression, Expression, transformSync, ImportDeclaration, ImportNamespaceSpecifier, Module } from "@swc/core"
import Visitor from "@swc/core/Visitor"

type Options = {
    libraryName: string
}

const isEmptyImportDecl = (importDecl: ImportDeclaration) => !importDecl.specifiers.length

export default class UILibImporter extends Visitor {
    options: Options
    importItems: ImportDeclaration[] = []
    constructor(options: Options) {
        super()
        this.options = options
    }

    visitModule(m: Module) {
        m = super.visitModule(m)
        for (let i = m.body.length - 1;i >= 0;i--) {
            const item = m.body[i]
             if (item.type === 'ImportDeclaration') {
                const importDecl = item as ImportDeclaration
                if (isEmptyImportDecl(importDecl)) {
                    m.body.splice(i, 1)
                }
             }
        }
        m.body.unshift(...this.importItems)
        return m
    }

    visitImportDeclaration(n: ImportDeclaration) {
        if (n.source.value === this.options.libraryName) {
            for (let i = n.specifiers.length - 1;i >= 0;i--) {
               const specifier = n.specifiers[i]
                if (specifier.type === 'ImportNamespaceSpecifier') {
                    this.importItems.push({
                        type: 'ImportDeclaration',
                        specifiers: [{
                            type: 'ImportDefaultSpecifier',
                            local: specifier.local,
                            span: specifier.span
                        }],
                        span: specifier.span,
                        source: {
                            type: 'StringLiteral',
                            span:  n.source.span,
                            value: `${n.source.value}/lib/${specifier.local.value}`,
                            has_escape: n.source.has_escape
                        }
                    })
                    n.specifiers.splice(i, 1)
                }
            }
            return n
        }
        return super.visitImportDeclaration(n)
    }
}