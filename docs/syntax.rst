..
   # *******************************************************************************
   # Copyright (c) 2025 Contributors to the Eclipse Foundation
   #
   # See the NOTICE file(s) distributed with this work for additional
   # information regarding copyright ownership.
   #
   # This program and the accompanying materials are made available under the
   # terms of the Apache License Version 2.0 which is available at
   # https://www.apache.org/licenses/LICENSE-2.0
   #
   # SPDX-License-Identifier: Apache-2.0
   # *******************************************************************************

Type Description Syntax
#######################

Syntactical Rules
=================

The notation ``▢,+`` denotes a comma-separated list of *one or more* ``▢`` with an optional comma
at the end.
Examples:

* ``▢``
* ``▢,``
* ``▢,▢``
* ``▢,▢,``

The notation ``▢,*`` denotes a comma-separated list of *zero or more* ``▢``,
with an optional comma at the end if the list is non-empty.
Examples:

* `` ``
* ``▢``
* ``▢,``
* ``▢,▢``
* ``▢,▢,``

::

   SourceFile →
      Module

   Module →
      InnerDoc? Item*

   Item →
      Import | ModuleDecl | TypeDecl

   Import →
      OuterDoc? Attribute* 'use' Path ';'

   Path →
      '::'? ( Identifier '::' )* Identifier

   Attribute →
      '#' '[' Identifier ( '=' Path )? ']'

   GenericParams →
      '<' ( Identifier | 'const' Identifier ),* '>'

   GenericArgs →
      '<' ( TypeRef | Integer ),* '>'

   TypeRef →
      Path GenericArgs?

   ModuleDecl →
         OuterDoc? Attribute* 'mod' Identifier ';'
      | OuterDoc? Attribute* 'mod' Identifier '{' Module '}'

   TypeDecl →
         OuterDoc? Attribute* 'type' Identifier GenericParams? '=' TypeRef ';'
      | OuterDoc? Attribute* 'struct' Identifier GenericParams? '(' TypeRef,+ ')'
      | OuterDoc? Attribute* 'struct' Identifier GenericParams? '{' StructField,+ '}'
      | OuterDoc? Attribute* 'enum' Identifier GenericParams? '{' EnumVariant,+ '}'

   StructField →
      OuterDoc? Identifier ':' TypeRef

   EnumVariant →
         OuterDoc? Identifier
      | OuterDoc? Identifier '(' TypeRef,+ ')'
      | OuterDoc? Identifier '{' StructField,+ '}'

Lexical Rules
=============

::

   Identifier → [a-zA-Z_][a-zA-Z0-9_]*

   Integer → '-'? [0-9]+

   InnerDoc → '//!'…

   OuterDoc → '///'…
