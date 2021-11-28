import { Identifier, ImportDeclaration, KeyValuePatternProperty, Module, ModuleItem, Statement, VariableDeclaration } from "@swc/core"
import Visitor from "@swc/core/Visitor"


type CustomStylePath = (name: string) => string | false

type CustomName = (name: string) => string

type Options = {
    libraryName: string,
    style?: boolean | 'css' | CustomStylePath,
    styleLibraryDirectory?: string
    camel2DashComponentName?: boolean
    customName?: CustomName | string
    transformToDefaultImport?: boolean
}

const defaultOptions = {
    style: true,
    camel2DashComponentName: true,
}

const isEmptyImportDecl = (importDecl: ImportDeclaration) => !importDecl.specifiers.length

export default class UILibImporter extends Visitor {
    options: Options
    importItems: ImportDeclaration[] = []
    constructor(options: Options) {
        super()
        this.options = {...defaultOptions,  ...options}
    }

    visitModule(m: Module) {
        m = super.visitModule(m)
        for (let i = m.body.length - 1; i >= 0; i--) {
            const item = m.body[i]
            if (item.type === 'ImportDeclaration') {
                const importDecl = item as ImportDeclaration
                if (importDecl.source.value === this.options.libraryName &&
                    isEmptyImportDecl(importDecl)) {
                    m.body.splice(i, 1)
                }
            }
        }
        m.body.unshift(...this.importItems)
        return m
    }

    visitStatements(stmts: Statement[]) {
        stmts = super.visitStatements(stmts)
        for (let i = stmts.length - 1; i >= 0; i--) {
            const newStatements = this._visitStatement(stmts[i])
            if (!(newStatements.length === 1 && newStatements[0] === stmts[i])) {
                stmts.splice(i, 1, ...newStatements)
            }
        }
        return stmts
    }

    _visitStatement(s: Statement) {
        const ret: Statement[] = [s]
        if (s.type === 'VariableDeclaration') {
            for (let i = s.declarations.length - 1; i >= 0; i--) {
                const declaration = s.declarations[i]
                if (declaration.init) {
                    if (declaration.init.type === 'CallExpression' &&
                        declaration.init.callee.type === 'Identifier' &&
                        declaration.init.callee.value === 'require' &&
                        declaration.init.arguments.length &&
                        declaration.init.arguments[0].expression.type === 'StringLiteral' &&
                        declaration.init.arguments[0].expression.value === this.options.libraryName &&
                        declaration.id.type === 'ObjectPattern'
                    ) {
                        const properties = declaration.id.properties
                        for (let i = properties.length - 1; i >= 0; i--) {
                            const property = properties[i]
                            if ((property.type === 'KeyValuePatternProperty' && property.key.type === 'Identifier') || property.type === 'AssignmentPatternProperty') {
                                const key = property.type === 'KeyValuePatternProperty' ? property.key as Identifier : property.key
                                const value = property.type === 'KeyValuePatternProperty' ? property.value : property.key
                                ret.push({
                                    ...s,
                                    declarations: [{
                                        ...declaration,
                                        id: this.options.transformToDefaultImport !== false ?
                                            value
                                            : {
                                                ...declaration.id,
                                                properties: [
                                                    properties[i]
                                                ]
                                            },
                                        init: {
                                            ...declaration.init,
                                            arguments: [{
                                                expression: {
                                                    ...declaration.init.arguments[0].expression,
                                                    value: this.generateComponentPath(key.value)
                                                }
                                            }]
                                        }
                                    }]
                                })
                                if (this.options.style || this.options.styleLibraryDirectory) {
                                    const stylePath = this.generateStyleSource(key.value)
                                    if (stylePath) {
                                        ret.push({
                                            type: 'ExpressionStatement',
                                            span: declaration.span,
                                            expression: {
                                                ...declaration.init,
                                                arguments: [{
                                                    expression: {
                                                        ...declaration.init.arguments[0].expression,
                                                        value: stylePath
                                                    }
                                                }]
                                            }
                                        })
                                    }
                                }
                                properties.splice(i, 1)
                            }
                        }
                        if (!properties.length) {
                            s.declarations.splice(i, 1)
                        }
                    }
                }
            }
            if (!s.declarations.length) {
                ret.shift()
            }

        }
        return ret
    }

    visitImportDeclaration(n: ImportDeclaration) {
        if (n.source.value === this.options.libraryName) {
            for (let i = n.specifiers.length - 1; i >= 0; i--) {
                const specifier = n.specifiers[i]
                if (specifier.type === 'ImportSpecifier') {
                    const imported = specifier.imported || specifier.local
                    const componentPath = this.generateComponentPath(imported.value)
                    this.importItems.push({
                        ...n,
                        specifiers: this.options.transformToDefaultImport !== false ? [{
                            ...specifier,
                            type: 'ImportDefaultSpecifier',
                        }] : [specifier],
                        source: {
                            ...n.source,
                            value: componentPath
                        }
                    })
                    if (this.options.style || this.options.styleLibraryDirectory) {
                        const stylePath = this.generateStyleSource(imported.value)
                        if (stylePath) {
                            this.importItems.push({
                                type: 'ImportDeclaration',
                                specifiers: [],
                                span: specifier.span,
                                source: {
                                    ...n.source,
                                    value: stylePath
                                }
                            })
                        }
                    }
                    n.specifiers.splice(i, 1)
                }
            }
            return n
        }
        return super.visitImportDeclaration(n)
    }

    generateComponentPath(source: string) {
        if (this.options.customName) {
            if (typeof this.options.customName === 'function') {
                return this.options.customName(source)
            } else {
                return (require(this.options.customName).customName as CustomName)(source)
            }
        }
        return `${this.options.libraryName}/lib/${this.generateComponentName(source)}`
    }

    generateStyleSource(source: string) {
        if (typeof this.options.style === 'function') {
            return (this.options.style as CustomStylePath).call(null, source)
        }
        if (this.options.styleLibraryDirectory) {
            return `${this.options.libraryName}/${this.options.styleLibraryDirectory}/${this.generateComponentName(source)}`
        } else if (this.options.style) {
            const s = this.options.style ? 'style' : 'style/css'
            return `${this.generateComponentPath(source)}/${s}`
        }
    }

    generateComponentName(source: string) {
        return this.options.camel2DashComponentName ?
            source.replace(/(?<=[a-z])([A-Z])/g, (m) => `-${m.toLowerCase()}`).replace(/^[A-Z]/, (m) => m.toLowerCase()) :
            source
    }
}