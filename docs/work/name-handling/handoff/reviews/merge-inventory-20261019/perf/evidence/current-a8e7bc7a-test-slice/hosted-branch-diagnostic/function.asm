
/workspace/tcl-lsp-perf-latest/target/debug/deps/tcl_compiler-455d0126ba4b4f9f:     file format elf64-x86-64


Disassembly of section .text:

00000000050e25a0 <<tcl_compiler::lowering::Lowerer>::original_when_event_word>:
 50e25a0:	48 81 ec 00 10 00 00 	sub    $0x1000,%rsp
 50e25a7:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
 50e25ae:	00 
 50e25af:	48 81 ec 08 04 00 00 	sub    $0x408,%rsp
 50e25b6:	48 89 94 24 50 03 00 	mov    %rdx,0x350(%rsp)
 50e25bd:	00 
 50e25be:	48 89 b4 24 58 03 00 	mov    %rsi,0x358(%rsp)
 50e25c5:	00 
 50e25c6:	48 89 f8             	mov    %rdi,%rax
 50e25c9:	48 8b bc 24 58 03 00 	mov    0x358(%rsp),%rdi
 50e25d0:	00 
 50e25d1:	48 89 84 24 60 03 00 	mov    %rax,0x360(%rsp)
 50e25d8:	00 
 50e25d9:	48 89 84 24 68 03 00 	mov    %rax,0x368(%rsp)
 50e25e0:	00 
 50e25e1:	48 89 8c 24 70 03 00 	mov    %rcx,0x370(%rsp)
 50e25e8:	00 
 50e25e9:	4c 89 84 24 78 03 00 	mov    %r8,0x378(%rsp)
 50e25f0:	00 
 50e25f1:	4c 89 8c 24 80 03 00 	mov    %r9,0x380(%rsp)
 50e25f8:	00 
 50e25f9:	48 81 c7 20 09 00 00 	add    $0x920,%rdi
 50e2600:	e8 1b 77 71 ff       	call   47f9d20 <<core::option::Option<tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory>>::as_ref>
 50e2605:	48 89 c7             	mov    %rax,%rdi
 50e2608:	48 8d 35 5d 73 d3 fb 	lea    -0x42c8ca3(%rip),%rsi        # e1996c <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153a5c>
 50e260f:	ba 0f 00 00 00       	mov    $0xf,%edx
 50e2614:	e8 17 46 e2 ff       	call   4f06c30 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<&tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory>>
 50e2619:	48 89 c7             	mov    %rax,%rdi
 50e261c:	e8 ef 2f 8a ff       	call   4985610 <<core::option::Option<&tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory> as core::ops::try_trait::Try>::branch>
 50e2621:	48 89 84 24 88 03 00 	mov    %rax,0x388(%rsp)
 50e2628:	00 
 50e2629:	48 8b 94 24 88 03 00 	mov    0x388(%rsp),%rdx
 50e2630:	00 
 50e2631:	31 c0                	xor    %eax,%eax
 50e2633:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e2638:	48 83 fa 00          	cmp    $0x0,%rdx
 50e263c:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2640:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2646:	74 12                	je     50e265a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xba>
 50e2648:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e264f:	00 
 50e2650:	e8 8b d3 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2655:	e9 ad 16 00 00       	jmp    50e3d07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1767>
 50e265a:	48 8b bc 24 88 03 00 	mov    0x388(%rsp),%rdi
 50e2661:	00 
 50e2662:	48 89 bc 24 40 03 00 	mov    %rdi,0x340(%rsp)
 50e2669:	00 
 50e266a:	e8 01 1b de ff       	call   4ec4170 <<tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory as core::ops::deref::Deref>::deref>
 50e266f:	48 89 c1             	mov    %rax,%rcx
 50e2672:	48 8b 84 24 50 03 00 	mov    0x350(%rsp),%rax
 50e2679:	00 
 50e267a:	48 89 8c 24 48 03 00 	mov    %rcx,0x348(%rsp)
 50e2681:	00 
 50e2682:	8b b8 a8 00 00 00    	mov    0xa8(%rax),%edi
 50e2688:	8b b0 ac 00 00 00    	mov    0xac(%rax),%esi
 50e268e:	ff 15 c4 27 83 03    	call   *0x38327c4(%rip)        # 8914e58 <_DYNAMIC+0x1e98>
 50e2694:	48 8b bc 24 48 03 00 	mov    0x348(%rsp),%rdi
 50e269b:	00 
 50e269c:	89 c6                	mov    %eax,%esi
 50e269e:	e8 bd 05 df fd       	call   2ed2c60 <<tcl_compiler::command_binding::SourceCommandBindings>::deferred_rule_declaration_candidates>
 50e26a3:	48 89 c7             	mov    %rax,%rdi
 50e26a6:	48 89 d6             	mov    %rdx,%rsi
 50e26a9:	48 8d 15 cb 72 d3 fb 	lea    -0x42c8d35(%rip),%rdx        # e1997b <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153a6b>
 50e26b0:	b9 13 00 00 00       	mov    $0x13,%ecx
 50e26b5:	e8 c6 46 e2 ff       	call   4f06d80 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<&[tcl_compiler::command_binding::rule_declaration::SourceRuleDeclarationCandidate]>>
 50e26ba:	48 89 c7             	mov    %rax,%rdi
 50e26bd:	48 89 d6             	mov    %rdx,%rsi
 50e26c0:	e8 0b 54 8a ff       	call   4987ad0 <<core::option::Option<&[tcl_compiler::command_binding::rule_declaration::SourceRuleDeclarationCandidate]> as core::ops::try_trait::Try>::branch>
 50e26c5:	48 89 84 24 90 03 00 	mov    %rax,0x390(%rsp)
 50e26cc:	00 
 50e26cd:	48 89 94 24 98 03 00 	mov    %rdx,0x398(%rsp)
 50e26d4:	00 
 50e26d5:	48 8b 94 24 90 03 00 	mov    0x390(%rsp),%rdx
 50e26dc:	00 
 50e26dd:	31 c0                	xor    %eax,%eax
 50e26df:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e26e4:	48 83 fa 00          	cmp    $0x0,%rdx
 50e26e8:	48 0f 44 c1          	cmove  %rcx,%rax
 50e26ec:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e26f2:	74 12                	je     50e2706 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x166>
 50e26f4:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e26fb:	00 
 50e26fc:	e8 df d2 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2701:	e9 01 16 00 00       	jmp    50e3d07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1767>
 50e2706:	48 8b bc 24 90 03 00 	mov    0x390(%rsp),%rdi
 50e270d:	00 
 50e270e:	48 89 bc 24 30 03 00 	mov    %rdi,0x330(%rsp)
 50e2715:	00 
 50e2716:	48 8b b4 24 98 03 00 	mov    0x398(%rsp),%rsi
 50e271d:	00 
 50e271e:	48 89 b4 24 38 03 00 	mov    %rsi,0x338(%rsp)
 50e2725:	00 
 50e2726:	e8 8d 19 52 00       	call   56040b8 <<[tcl_compiler::command_binding::rule_declaration::SourceRuleDeclarationCandidate]>::iter>
 50e272b:	48 89 c6             	mov    %rax,%rsi
 50e272e:	48 8b 84 24 58 03 00 	mov    0x358(%rsp),%rax
 50e2735:	00 
 50e2736:	4c 8b 80 e8 08 00 00 	mov    0x8e8(%rax),%r8
 50e273d:	48 8d bc 24 00 04 00 	lea    0x400(%rsp),%rdi
 50e2744:	00 
 50e2745:	48 8d 8c 24 70 03 00 	lea    0x370(%rsp),%rcx
 50e274c:	00 
 50e274d:	e8 ce 4e 53 fd       	call   2617620 <<core::slice::iter::Iter<tcl_compiler::command_binding::rule_declaration::SourceRuleDeclarationCandidate> as core::iter::traits::iterator::Iterator>::map::<core::option::Option<usize>, <tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#0}>>
 50e2752:	48 8d bc 24 e8 03 00 	lea    0x3e8(%rsp),%rdi
 50e2759:	00 
 50e275a:	48 8d b4 24 00 04 00 	lea    0x400(%rsp),%rsi
 50e2761:	00 
 50e2762:	e8 b9 ba 9b ff       	call   4a9e220 <<core::iter::adapters::map::Map<core::slice::iter::Iter<tcl_compiler::command_binding::rule_declaration::SourceRuleDeclarationCandidate>, <tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#0}> as core::iter::traits::iterator::Iterator>::collect::<core::option::Option<alloc::vec::Vec<usize>>>>
 50e2767:	48 8d bc 24 d0 03 00 	lea    0x3d0(%rsp),%rdi
 50e276e:	00 
 50e276f:	48 8d b4 24 e8 03 00 	lea    0x3e8(%rsp),%rsi
 50e2776:	00 
 50e2777:	48 8d 15 10 72 d3 fb 	lea    -0x42c8df0(%rip),%rdx        # e1998e <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153a7e>
 50e277e:	b9 0d 00 00 00       	mov    $0xd,%ecx
 50e2783:	e8 88 3e e2 ff       	call   4f06610 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<alloc::vec::Vec<usize>>>
 50e2788:	48 8d bc 24 b8 03 00 	lea    0x3b8(%rsp),%rdi
 50e278f:	00 
 50e2790:	48 8d b4 24 d0 03 00 	lea    0x3d0(%rsp),%rsi
 50e2797:	00 
 50e2798:	ff 15 ca 2f 83 03    	call   *0x3832fca(%rip)        # 8915768 <_DYNAMIC+0x27a8>
 50e279e:	31 c0                	xor    %eax,%eax
 50e27a0:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e27a5:	48 83 bc 24 b8 03 00 	cmpq   $0xffffffffffffffff,0x3b8(%rsp)
 50e27ac:	00 ff 
 50e27ae:	48 0f 44 c1          	cmove  %rcx,%rax
 50e27b2:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e27b8:	74 12                	je     50e27cc <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x22c>
 50e27ba:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e27c1:	00 
 50e27c2:	e8 19 d2 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e27c7:	e9 3b 15 00 00       	jmp    50e3d07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1767>
 50e27cc:	48 8b 84 24 c8 03 00 	mov    0x3c8(%rsp),%rax
 50e27d3:	00 
 50e27d4:	48 89 84 24 30 04 00 	mov    %rax,0x430(%rsp)
 50e27db:	00 
 50e27dc:	0f 10 84 24 b8 03 00 	movups 0x3b8(%rsp),%xmm0
 50e27e3:	00 
 50e27e4:	0f 29 84 24 20 04 00 	movaps %xmm0,0x420(%rsp)
 50e27eb:	00 
 50e27ec:	48 8b 84 24 30 04 00 	mov    0x430(%rsp),%rax
 50e27f3:	00 
 50e27f4:	48 89 84 24 b0 03 00 	mov    %rax,0x3b0(%rsp)
 50e27fb:	00 
 50e27fc:	0f 28 84 24 20 04 00 	movaps 0x420(%rsp),%xmm0
 50e2803:	00 
 50e2804:	0f 29 84 24 a0 03 00 	movaps %xmm0,0x3a0(%rsp)
 50e280b:	00 
 50e280c:	48 8b 05 55 2b 83 03 	mov    0x3832b55(%rip),%rax        # 8915368 <_DYNAMIC+0x23a8>
 50e2813:	48 8d bc 24 a0 03 00 	lea    0x3a0(%rsp),%rdi
 50e281a:	00 
 50e281b:	ff d0                	call   *%rax
 50e281d:	48 89 94 24 20 03 00 	mov    %rdx,0x320(%rsp)
 50e2824:	00 
 50e2825:	48 89 84 24 28 03 00 	mov    %rax,0x328(%rsp)
 50e282c:	00 
 50e282d:	eb 2c                	jmp    50e285b <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x2bb>
 50e282f:	48 8b 05 6a 0c 83 03 	mov    0x3830c6a(%rip),%rax        # 89134a0 <_DYNAMIC+0x4e0>
 50e2836:	48 8d bc 24 a0 03 00 	lea    0x3a0(%rsp),%rdi
 50e283d:	00 
 50e283e:	ff d0                	call   *%rax
 50e2840:	e9 09 15 00 00       	jmp    50e3d4e <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x17ae>
 50e2845:	48 89 c1             	mov    %rax,%rcx
 50e2848:	89 d0                	mov    %edx,%eax
 50e284a:	48 89 8c 24 60 13 00 	mov    %rcx,0x1360(%rsp)
 50e2851:	00 
 50e2852:	89 84 24 68 13 00 00 	mov    %eax,0x1368(%rsp)
 50e2859:	eb d4                	jmp    50e282f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x28f>
 50e285b:	48 8b b4 24 20 03 00 	mov    0x320(%rsp),%rsi
 50e2862:	00 
 50e2863:	48 8b bc 24 28 03 00 	mov    0x328(%rsp),%rdi
 50e286a:	00 
 50e286b:	48 8b 05 2e 6a 83 03 	mov    0x3836a2e(%rip),%rax        # 89192a0 <_DYNAMIC+0x62e0>
 50e2872:	ff d0                	call   *%rax
 50e2874:	48 89 84 24 18 03 00 	mov    %rax,0x318(%rsp)
 50e287b:	00 
 50e287c:	eb 00                	jmp    50e287e <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x2de>
 50e287e:	48 8b bc 24 18 03 00 	mov    0x318(%rsp),%rdi
 50e2885:	00 
 50e2886:	48 8d 35 0e 71 d3 fb 	lea    -0x42c8ef2(%rip),%rsi        # e1999b <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153a8b>
 50e288d:	ba 0b 00 00 00       	mov    $0xb,%edx
 50e2892:	e8 89 47 e2 ff       	call   4f07020 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<&usize>>
 50e2897:	48 89 84 24 10 03 00 	mov    %rax,0x310(%rsp)
 50e289e:	00 
 50e289f:	eb 00                	jmp    50e28a1 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x301>
 50e28a1:	48 8b bc 24 10 03 00 	mov    0x310(%rsp),%rdi
 50e28a8:	00 
 50e28a9:	48 8b 05 c8 6f 83 03 	mov    0x3836fc8(%rip),%rax        # 8919878 <_DYNAMIC+0x68b8>
 50e28b0:	ff d0                	call   *%rax
 50e28b2:	48 89 84 24 08 03 00 	mov    %rax,0x308(%rsp)
 50e28b9:	00 
 50e28ba:	eb 00                	jmp    50e28bc <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x31c>
 50e28bc:	48 8b 84 24 08 03 00 	mov    0x308(%rsp),%rax
 50e28c3:	00 
 50e28c4:	48 89 84 24 40 04 00 	mov    %rax,0x440(%rsp)
 50e28cb:	00 
 50e28cc:	48 8b 94 24 40 04 00 	mov    0x440(%rsp),%rdx
 50e28d3:	00 
 50e28d4:	31 c0                	xor    %eax,%eax
 50e28d6:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e28db:	48 83 fa 00          	cmp    $0x0,%rdx
 50e28df:	48 0f 44 c1          	cmove  %rcx,%rax
 50e28e3:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e28e9:	74 12                	je     50e28fd <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x35d>
 50e28eb:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e28f2:	00 
 50e28f3:	e8 e8 d0 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e28f8:	e9 4f 14 00 00       	jmp    50e3d4c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x17ac>
 50e28fd:	48 8b 84 24 40 04 00 	mov    0x440(%rsp),%rax
 50e2904:	00 
 50e2905:	48 8b 00             	mov    (%rax),%rax
 50e2908:	48 89 84 24 38 04 00 	mov    %rax,0x438(%rsp)
 50e290f:	00 
 50e2910:	48 8b 05 51 2a 83 03 	mov    0x3832a51(%rip),%rax        # 8915368 <_DYNAMIC+0x23a8>
 50e2917:	48 8d bc 24 a0 03 00 	lea    0x3a0(%rsp),%rdi
 50e291e:	00 
 50e291f:	ff d0                	call   *%rax
 50e2921:	48 89 94 24 f8 02 00 	mov    %rdx,0x2f8(%rsp)
 50e2928:	00 
 50e2929:	48 89 84 24 00 03 00 	mov    %rax,0x300(%rsp)
 50e2930:	00 
 50e2931:	eb 00                	jmp    50e2933 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x393>
 50e2933:	48 8b b4 24 f8 02 00 	mov    0x2f8(%rsp),%rsi
 50e293a:	00 
 50e293b:	48 8b bc 24 00 03 00 	mov    0x300(%rsp),%rdi
 50e2942:	00 
 50e2943:	48 8b 05 86 23 83 03 	mov    0x3832386(%rip),%rax        # 8914cd0 <_DYNAMIC+0x1d10>
 50e294a:	ff d0                	call   *%rax
 50e294c:	48 89 94 24 e8 02 00 	mov    %rdx,0x2e8(%rsp)
 50e2953:	00 
 50e2954:	48 89 84 24 f0 02 00 	mov    %rax,0x2f0(%rsp)
 50e295b:	00 
 50e295c:	eb 00                	jmp    50e295e <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x3be>
 50e295e:	48 8b 84 24 e8 02 00 	mov    0x2e8(%rsp),%rax
 50e2965:	00 
 50e2966:	48 8b 8c 24 f0 02 00 	mov    0x2f0(%rsp),%rcx
 50e296d:	00 
 50e296e:	48 89 8c 24 48 04 00 	mov    %rcx,0x448(%rsp)
 50e2975:	00 
 50e2976:	48 89 84 24 50 04 00 	mov    %rax,0x450(%rsp)
 50e297d:	00 
 50e297e:	48 8d bc 24 48 04 00 	lea    0x448(%rsp),%rdi
 50e2985:	00 
 50e2986:	48 8d b4 24 38 04 00 	lea    0x438(%rsp),%rsi
 50e298d:	00 
 50e298e:	e8 ad 3a 4f fd       	call   25d6440 <<core::slice::iter::Iter<usize> as core::iter::traits::iterator::Iterator>::any::<<tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#1}>>
 50e2993:	88 84 24 e7 02 00 00 	mov    %al,0x2e7(%rsp)
 50e299a:	eb 00                	jmp    50e299c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x3fc>
 50e299c:	8a 84 24 e7 02 00 00 	mov    0x2e7(%rsp),%al
 50e29a3:	a8 01                	test   $0x1,%al
 50e29a5:	75 21                	jne    50e29c8 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x428>
 50e29a7:	eb 00                	jmp    50e29a9 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x409>
 50e29a9:	48 8b 94 24 50 03 00 	mov    0x350(%rsp),%rdx
 50e29b0:	00 
 50e29b1:	48 8b b4 24 58 03 00 	mov    0x358(%rsp),%rsi
 50e29b8:	00 
 50e29b9:	48 8d bc 24 58 04 00 	lea    0x458(%rsp),%rdi
 50e29c0:	00 
 50e29c1:	e8 9a af fe ff       	call   50cd960 <<tcl_compiler::lowering::Lowerer>::cmd_tokens>
 50e29c6:	eb 14                	jmp    50e29dc <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x43c>
 50e29c8:	48 8b 84 24 60 03 00 	mov    0x360(%rsp),%rax
 50e29cf:	00 
 50e29d0:	48 c7 00 ff ff ff ff 	movq   $0xffffffffffffffff,(%rax)
 50e29d7:	e9 60 13 00 00       	jmp    50e3d3c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x179c>
 50e29dc:	48 8d bc 24 78 0a 00 	lea    0xa78(%rsp),%rdi
 50e29e3:	00 
 50e29e4:	e8 d7 19 84 fd       	call   29243c0 <<alloc::vec::Vec<tcl_compiler::ir::WordExpr> as core::ops::deref::Deref>::deref>
 50e29e9:	48 89 94 24 d0 02 00 	mov    %rdx,0x2d0(%rsp)
 50e29f0:	00 
 50e29f1:	48 89 84 24 d8 02 00 	mov    %rax,0x2d8(%rsp)
 50e29f8:	00 
 50e29f9:	eb 28                	jmp    50e2a23 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x483>
 50e29fb:	48 8d bc 24 58 04 00 	lea    0x458(%rsp),%rdi
 50e2a02:	00 
 50e2a03:	e8 18 9e ea ff       	call   4f8c820 <core::ptr::drop_glue::<tcl_compiler::ir::CommandTokens>>
 50e2a08:	e9 22 fe ff ff       	jmp    50e282f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x28f>
 50e2a0d:	48 89 c1             	mov    %rax,%rcx
 50e2a10:	89 d0                	mov    %edx,%eax
 50e2a12:	48 89 8c 24 60 13 00 	mov    %rcx,0x1360(%rsp)
 50e2a19:	00 
 50e2a1a:	89 84 24 68 13 00 00 	mov    %eax,0x1368(%rsp)
 50e2a21:	eb d8                	jmp    50e29fb <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x45b>
 50e2a23:	48 8b 84 24 50 03 00 	mov    0x350(%rsp),%rax
 50e2a2a:	00 
 50e2a2b:	48 8b 8c 24 d0 02 00 	mov    0x2d0(%rsp),%rcx
 50e2a32:	00 
 50e2a33:	48 8b 94 24 d8 02 00 	mov    0x2d8(%rsp),%rdx
 50e2a3a:	00 
 50e2a3b:	48 89 94 24 b8 02 00 	mov    %rdx,0x2b8(%rsp)
 50e2a42:	00 
 50e2a43:	48 89 8c 24 c0 02 00 	mov    %rcx,0x2c0(%rsp)
 50e2a4a:	00 
 50e2a4b:	8b b8 a8 00 00 00    	mov    0xa8(%rax),%edi
 50e2a51:	8b b0 ac 00 00 00    	mov    0xac(%rax),%esi
 50e2a57:	48 8b 05 fa 23 83 03 	mov    0x38323fa(%rip),%rax        # 8914e58 <_DYNAMIC+0x1e98>
 50e2a5e:	ff d0                	call   *%rax
 50e2a60:	89 84 24 cc 02 00 00 	mov    %eax,0x2cc(%rsp)
 50e2a67:	eb 00                	jmp    50e2a69 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x4c9>
 50e2a69:	44 8b 84 24 cc 02 00 	mov    0x2cc(%rsp),%r8d
 50e2a70:	00 
 50e2a71:	48 8b 8c 24 c0 02 00 	mov    0x2c0(%rsp),%rcx
 50e2a78:	00 
 50e2a79:	48 8b 94 24 b8 02 00 	mov    0x2b8(%rsp),%rdx
 50e2a80:	00 
 50e2a81:	48 8b 84 24 58 03 00 	mov    0x358(%rsp),%rax
 50e2a88:	00 
 50e2a89:	0f 10 80 c8 09 00 00 	movups 0x9c8(%rax),%xmm0
 50e2a90:	0f 10 88 d4 09 00 00 	movups 0x9d4(%rax),%xmm1
 50e2a97:	0f 11 8c 24 bc 0b 00 	movups %xmm1,0xbbc(%rsp)
 50e2a9e:	00 
 50e2a9f:	0f 29 84 24 b0 0b 00 	movaps %xmm0,0xbb0(%rsp)
 50e2aa6:	00 
 50e2aa7:	48 8b b4 24 80 03 00 	mov    0x380(%rsp),%rsi
 50e2aae:	00 
 50e2aaf:	48 8d bc 24 98 0b 00 	lea    0xb98(%rsp),%rdi
 50e2ab6:	00 
 50e2ab7:	4c 8d 8c 24 b0 0b 00 	lea    0xbb0(%rsp),%r9
 50e2abe:	00 
 50e2abf:	e8 bc da c2 fd       	call   2d10580 <tcl_compiler::registry_invocation::native_compilation_source::original_native_compiler_words>
 50e2ac4:	eb 00                	jmp    50e2ac6 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x526>
 50e2ac6:	48 8d 15 d9 6e d3 fb 	lea    -0x42c9127(%rip),%rdx        # e199a6 <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153a96>
 50e2acd:	48 8d bc 24 80 0b 00 	lea    0xb80(%rsp),%rdi
 50e2ad4:	00 
 50e2ad5:	48 8d b4 24 98 0b 00 	lea    0xb98(%rsp),%rsi
 50e2adc:	00 
 50e2add:	b9 0e 00 00 00       	mov    $0xe,%ecx
 50e2ae2:	e8 79 39 e2 ff       	call   4f06460 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<alloc::vec::Vec<tcl_lexer::native_word::NativeWord>>>
 50e2ae7:	eb 00                	jmp    50e2ae9 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x549>
 50e2ae9:	48 8d bc 24 68 0b 00 	lea    0xb68(%rsp),%rdi
 50e2af0:	00 
 50e2af1:	48 8d b4 24 80 0b 00 	lea    0xb80(%rsp),%rsi
 50e2af8:	00 
 50e2af9:	e8 42 81 88 ff       	call   496ac40 <<core::option::Option<alloc::vec::Vec<tcl_lexer::native_word::NativeWord>> as core::ops::try_trait::Try>::branch>
 50e2afe:	eb 00                	jmp    50e2b00 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x560>
 50e2b00:	31 c0                	xor    %eax,%eax
 50e2b02:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e2b07:	48 83 bc 24 68 0b 00 	cmpq   $0xffffffffffffffff,0xb68(%rsp)
 50e2b0e:	00 ff 
 50e2b10:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2b14:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2b1a:	74 12                	je     50e2b2e <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x58e>
 50e2b1c:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e2b23:	00 
 50e2b24:	e8 b7 ce 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2b29:	e9 0c 12 00 00       	jmp    50e3d3a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x179a>
 50e2b2e:	48 8b 84 24 78 0b 00 	mov    0xb78(%rsp),%rax
 50e2b35:	00 
 50e2b36:	48 89 84 24 e0 0b 00 	mov    %rax,0xbe0(%rsp)
 50e2b3d:	00 
 50e2b3e:	0f 10 84 24 68 0b 00 	movups 0xb68(%rsp),%xmm0
 50e2b45:	00 
 50e2b46:	0f 29 84 24 d0 0b 00 	movaps %xmm0,0xbd0(%rsp)
 50e2b4d:	00 
 50e2b4e:	48 8b 84 24 e0 0b 00 	mov    0xbe0(%rsp),%rax
 50e2b55:	00 
 50e2b56:	48 89 84 24 60 0b 00 	mov    %rax,0xb60(%rsp)
 50e2b5d:	00 
 50e2b5e:	0f 28 84 24 d0 0b 00 	movaps 0xbd0(%rsp),%xmm0
 50e2b65:	00 
 50e2b66:	0f 29 84 24 50 0b 00 	movaps %xmm0,0xb50(%rsp)
 50e2b6d:	00 
 50e2b6e:	48 8b 05 0b 29 83 03 	mov    0x383290b(%rip),%rax        # 8915480 <_DYNAMIC+0x24c0>
 50e2b75:	48 8d bc 24 50 0b 00 	lea    0xb50(%rsp),%rdi
 50e2b7c:	00 
 50e2b7d:	ff d0                	call   *%rax
 50e2b7f:	48 89 94 24 a8 02 00 	mov    %rdx,0x2a8(%rsp)
 50e2b86:	00 
 50e2b87:	48 89 84 24 b0 02 00 	mov    %rax,0x2b0(%rsp)
 50e2b8e:	00 
 50e2b8f:	eb 2c                	jmp    50e2bbd <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x61d>
 50e2b91:	48 8b 05 18 17 83 03 	mov    0x3831718(%rip),%rax        # 89142b0 <_DYNAMIC+0x12f0>
 50e2b98:	48 8d bc 24 50 0b 00 	lea    0xb50(%rsp),%rdi
 50e2b9f:	00 
 50e2ba0:	ff d0                	call   *%rax
 50e2ba2:	e9 54 fe ff ff       	jmp    50e29fb <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x45b>
 50e2ba7:	48 89 c1             	mov    %rax,%rcx
 50e2baa:	89 d0                	mov    %edx,%eax
 50e2bac:	48 89 8c 24 60 13 00 	mov    %rcx,0x1360(%rsp)
 50e2bb3:	00 
 50e2bb4:	89 84 24 68 13 00 00 	mov    %eax,0x1368(%rsp)
 50e2bbb:	eb d4                	jmp    50e2b91 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x5f1>
 50e2bbd:	48 8b b4 24 a8 02 00 	mov    0x2a8(%rsp),%rsi
 50e2bc4:	00 
 50e2bc5:	48 8b bc 24 b0 02 00 	mov    0x2b0(%rsp),%rdi
 50e2bcc:	00 
 50e2bcd:	48 8b 94 24 38 04 00 	mov    0x438(%rsp),%rdx
 50e2bd4:	00 
 50e2bd5:	48 8b 05 fc 2e 83 03 	mov    0x3832efc(%rip),%rax        # 8915ad8 <_DYNAMIC+0x2b18>
 50e2bdc:	ff d0                	call   *%rax
 50e2bde:	48 89 84 24 a0 02 00 	mov    %rax,0x2a0(%rsp)
 50e2be5:	00 
 50e2be6:	eb 00                	jmp    50e2be8 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x648>
 50e2be8:	48 8b bc 24 a0 02 00 	mov    0x2a0(%rsp),%rdi
 50e2bef:	00 
 50e2bf0:	48 8d 35 bd 6d d3 fb 	lea    -0x42c9243(%rip),%rsi        # e199b4 <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153aa4>
 50e2bf7:	ba 13 00 00 00       	mov    $0x13,%edx
 50e2bfc:	e8 df 3e e2 ff       	call   4f06ae0 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<&tcl_lexer::native_word::NativeWord>>
 50e2c01:	48 89 84 24 98 02 00 	mov    %rax,0x298(%rsp)
 50e2c08:	00 
 50e2c09:	eb 00                	jmp    50e2c0b <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x66b>
 50e2c0b:	48 8b bc 24 98 02 00 	mov    0x298(%rsp),%rdi
 50e2c12:	00 
 50e2c13:	48 8b 05 36 1d 83 03 	mov    0x3831d36(%rip),%rax        # 8914950 <_DYNAMIC+0x1990>
 50e2c1a:	ff d0                	call   *%rax
 50e2c1c:	48 89 84 24 90 02 00 	mov    %rax,0x290(%rsp)
 50e2c23:	00 
 50e2c24:	eb 00                	jmp    50e2c26 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x686>
 50e2c26:	48 8b 84 24 90 02 00 	mov    0x290(%rsp),%rax
 50e2c2d:	00 
 50e2c2e:	48 89 84 24 e8 0b 00 	mov    %rax,0xbe8(%rsp)
 50e2c35:	00 
 50e2c36:	48 8b 94 24 e8 0b 00 	mov    0xbe8(%rsp),%rdx
 50e2c3d:	00 
 50e2c3e:	31 c0                	xor    %eax,%eax
 50e2c40:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e2c45:	48 83 fa 00          	cmp    $0x0,%rdx
 50e2c49:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2c4d:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2c53:	74 12                	je     50e2c67 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x6c7>
 50e2c55:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e2c5c:	00 
 50e2c5d:	e8 7e cd 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2c62:	e9 bf 10 00 00       	jmp    50e3d26 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1786>
 50e2c67:	48 8b b4 24 58 03 00 	mov    0x358(%rsp),%rsi
 50e2c6e:	00 
 50e2c6f:	48 8b 84 24 e8 0b 00 	mov    0xbe8(%rsp),%rax
 50e2c76:	00 
 50e2c77:	48 89 84 24 88 02 00 	mov    %rax,0x288(%rsp)
 50e2c7e:	00 
 50e2c7f:	48 8d bc 24 08 0c 00 	lea    0xc08(%rsp),%rdi
 50e2c86:	00 
 50e2c87:	e8 84 3f 00 00       	call   50e6c10 <<tcl_compiler::lowering::Lowerer>::invocation_metadata_context>
 50e2c8c:	eb 00                	jmp    50e2c8e <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x6ee>
 50e2c8e:	48 8d 15 1b e0 a6 fb 	lea    -0x4591fe5(%rip),%rdx        # b50cb0 <GCC_except_table0+0x1990>
 50e2c95:	48 8d bc 24 f0 0b 00 	lea    0xbf0(%rsp),%rdi
 50e2c9c:	00 
 50e2c9d:	48 8d b4 24 08 0c 00 	lea    0xc08(%rsp),%rsi
 50e2ca4:	00 
 50e2ca5:	b9 10 00 00 00       	mov    $0x10,%ecx
 50e2caa:	e8 b1 3c e2 ff       	call   4f06960 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<tcl_compiler::registry_invocation::metadata_context::InvocationMetadataContext>>
 50e2caf:	eb 00                	jmp    50e2cb1 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x711>
 50e2cb1:	48 8b 94 24 f0 0b 00 	mov    0xbf0(%rsp),%rdx
 50e2cb8:	00 
 50e2cb9:	b8 01 00 00 00       	mov    $0x1,%eax
 50e2cbe:	31 c9                	xor    %ecx,%ecx
 50e2cc0:	48 83 fa 00          	cmp    $0x0,%rdx
 50e2cc4:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2cc8:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2cce:	74 57                	je     50e2d27 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x787>
 50e2cd0:	48 8b 84 24 00 0c 00 	mov    0xc00(%rsp),%rax
 50e2cd7:	00 
 50e2cd8:	48 89 84 24 30 0c 00 	mov    %rax,0xc30(%rsp)
 50e2cdf:	00 
 50e2ce0:	0f 10 84 24 f0 0b 00 	movups 0xbf0(%rsp),%xmm0
 50e2ce7:	00 
 50e2ce8:	0f 29 84 24 20 0c 00 	movaps %xmm0,0xc20(%rsp)
 50e2cef:	00 
 50e2cf0:	48 8b 84 24 30 0c 00 	mov    0xc30(%rsp),%rax
 50e2cf7:	00 
 50e2cf8:	48 89 84 24 80 13 00 	mov    %rax,0x1380(%rsp)
 50e2cff:	00 
 50e2d00:	0f 28 84 24 20 0c 00 	movaps 0xc20(%rsp),%xmm0
 50e2d07:	00 
 50e2d08:	0f 29 84 24 70 13 00 	movaps %xmm0,0x1370(%rsp)
 50e2d0f:	00 
 50e2d10:	48 8d bc 24 70 13 00 	lea    0x1370(%rsp),%rdi
 50e2d17:	00 
 50e2d18:	e8 ab 8d 48 fe       	call   356bac8 <<tcl_compiler::registry_invocation::metadata_context::InvocationMetadataContext>::source_analysis_input>
 50e2d1d:	48 89 84 24 80 02 00 	mov    %rax,0x280(%rsp)
 50e2d24:	00 
 50e2d25:	eb 53                	jmp    50e2d7a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x7da>
 50e2d27:	48 8b 84 24 58 03 00 	mov    0x358(%rsp),%rax
 50e2d2e:	00 
 50e2d2f:	0f 10 80 80 07 00 00 	movups 0x780(%rax),%xmm0
 50e2d36:	0f 10 88 90 07 00 00 	movups 0x790(%rax),%xmm1
 50e2d3d:	0f 10 90 a0 07 00 00 	movups 0x7a0(%rax),%xmm2
 50e2d44:	0f 29 94 24 b0 0f 00 	movaps %xmm2,0xfb0(%rsp)
 50e2d4b:	00 
 50e2d4c:	0f 29 8c 24 a0 0f 00 	movaps %xmm1,0xfa0(%rsp)
 50e2d53:	00 
 50e2d54:	0f 29 84 24 90 0f 00 	movaps %xmm0,0xf90(%rsp)
 50e2d5b:	00 
 50e2d5c:	48 8b 05 e5 30 83 03 	mov    0x38330e5(%rip),%rax        # 8915e48 <_DYNAMIC+0x2e88>
 50e2d63:	48 8d bc 24 60 0f 00 	lea    0xf60(%rsp),%rdi
 50e2d6a:	00 
 50e2d6b:	48 8d b4 24 90 0f 00 	lea    0xf90(%rsp),%rsi
 50e2d72:	00 
 50e2d73:	ff d0                	call   *%rax
 50e2d75:	e9 b3 0c 00 00       	jmp    50e3a2d <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x148d>
 50e2d7a:	48 8b bc 24 80 02 00 	mov    0x280(%rsp),%rdi
 50e2d81:	00 
 50e2d82:	e8 59 3b 62 ff       	call   47068e0 <<core::option::Option<&tcl_compiler::analyser::input::ResolvedAnalysisInput>>::is_some_and::<<tcl_compiler::analyser::input::ResolvedAnalysisInput>::has_hosted_source_name_context>>
 50e2d87:	88 84 24 7f 02 00 00 	mov    %al,0x27f(%rsp)
 50e2d8e:	eb 00                	jmp    50e2d90 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x7f0>
 50e2d90:	8a 84 24 7f 02 00 00 	mov    0x27f(%rsp),%al
 50e2d97:	a8 01                	test   $0x1,%al
 50e2d99:	75 02                	jne    50e2d9d <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x7fd>
 50e2d9b:	eb 8a                	jmp    50e2d27 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x787>
 50e2d9d:	48 8b 84 24 30 0c 00 	mov    0xc30(%rsp),%rax
 50e2da4:	00 
 50e2da5:	48 89 84 24 a0 13 00 	mov    %rax,0x13a0(%rsp)
 50e2dac:	00 
 50e2dad:	0f 28 84 24 20 0c 00 	movaps 0xc20(%rsp),%xmm0
 50e2db4:	00 
 50e2db5:	0f 29 84 24 90 13 00 	movaps %xmm0,0x1390(%rsp)
 50e2dbc:	00 
 50e2dbd:	48 8d bc 24 90 13 00 	lea    0x1390(%rsp),%rdi
 50e2dc4:	00 
 50e2dc5:	e8 fe 8c 48 fe       	call   356bac8 <<tcl_compiler::registry_invocation::metadata_context::InvocationMetadataContext>::source_analysis_input>
 50e2dca:	48 89 84 24 70 02 00 	mov    %rax,0x270(%rsp)
 50e2dd1:	00 
 50e2dd2:	eb 00                	jmp    50e2dd4 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x834>
 50e2dd4:	48 8b bc 24 70 02 00 	mov    0x270(%rsp),%rdi
 50e2ddb:	00 
 50e2ddc:	e8 af 33 8a ff       	call   4986190 <<core::option::Option<&tcl_compiler::analyser::input::ResolvedAnalysisInput> as core::ops::try_trait::Try>::branch>
 50e2de1:	48 89 84 24 68 02 00 	mov    %rax,0x268(%rsp)
 50e2de8:	00 
 50e2de9:	eb 00                	jmp    50e2deb <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x84b>
 50e2deb:	48 8b 84 24 68 02 00 	mov    0x268(%rsp),%rax
 50e2df2:	00 
 50e2df3:	48 89 84 24 38 0c 00 	mov    %rax,0xc38(%rsp)
 50e2dfa:	00 
 50e2dfb:	48 8b 94 24 38 0c 00 	mov    0xc38(%rsp),%rdx
 50e2e02:	00 
 50e2e03:	31 c0                	xor    %eax,%eax
 50e2e05:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e2e0a:	48 83 fa 00          	cmp    $0x0,%rdx
 50e2e0e:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2e12:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2e18:	74 12                	je     50e2e2c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x88c>
 50e2e1a:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e2e21:	00 
 50e2e22:	e8 b9 cb 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2e27:	e9 ff 0b 00 00       	jmp    50e3a2b <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x148b>
 50e2e2c:	48 8b bc 24 40 03 00 	mov    0x340(%rsp),%rdi
 50e2e33:	00 
 50e2e34:	48 8b 84 24 38 0c 00 	mov    0xc38(%rsp),%rax
 50e2e3b:	00 
 50e2e3c:	48 89 84 24 58 02 00 	mov    %rax,0x258(%rsp)
 50e2e43:	00 
 50e2e44:	e8 27 13 de ff       	call   4ec4170 <<tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory as core::ops::deref::Deref>::deref>
 50e2e49:	48 89 84 24 60 02 00 	mov    %rax,0x260(%rsp)
 50e2e50:	00 
 50e2e51:	eb 00                	jmp    50e2e53 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x8b3>
 50e2e53:	48 8b bc 24 60 02 00 	mov    0x260(%rsp),%rdi
 50e2e5a:	00 
 50e2e5b:	e8 10 d3 f4 fd       	call   3030170 <<tcl_compiler::command_binding::SourceCommandBindings>::original_analysis_owner>
 50e2e60:	48 89 84 24 50 02 00 	mov    %rax,0x250(%rsp)
 50e2e67:	00 
 50e2e68:	eb 00                	jmp    50e2e6a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x8ca>
 50e2e6a:	48 8b bc 24 50 02 00 	mov    0x250(%rsp),%rdi
 50e2e71:	00 
 50e2e72:	e8 89 d5 89 ff       	call   4980400 <<core::option::Option<&alloc::sync::Arc<tcl_compiler::command_binding::OriginalSourceAnalysisOwner>> as core::ops::try_trait::Try>::branch>
 50e2e77:	48 89 84 24 48 02 00 	mov    %rax,0x248(%rsp)
 50e2e7e:	00 
 50e2e7f:	eb 00                	jmp    50e2e81 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x8e1>
 50e2e81:	48 8b 84 24 48 02 00 	mov    0x248(%rsp),%rax
 50e2e88:	00 
 50e2e89:	48 89 84 24 40 0c 00 	mov    %rax,0xc40(%rsp)
 50e2e90:	00 
 50e2e91:	48 8b 94 24 40 0c 00 	mov    0xc40(%rsp),%rdx
 50e2e98:	00 
 50e2e99:	31 c0                	xor    %eax,%eax
 50e2e9b:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e2ea0:	48 83 fa 00          	cmp    $0x0,%rdx
 50e2ea4:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2ea8:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2eae:	74 12                	je     50e2ec2 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x922>
 50e2eb0:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e2eb7:	00 
 50e2eb8:	e8 23 cb 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2ebd:	e9 67 0b 00 00       	jmp    50e3a29 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1489>
 50e2ec2:	48 8b bc 24 40 03 00 	mov    0x340(%rsp),%rdi
 50e2ec9:	00 
 50e2eca:	48 8b 84 24 40 0c 00 	mov    0xc40(%rsp),%rax
 50e2ed1:	00 
 50e2ed2:	48 89 84 24 38 02 00 	mov    %rax,0x238(%rsp)
 50e2ed9:	00 
 50e2eda:	e8 91 12 de ff       	call   4ec4170 <<tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory as core::ops::deref::Deref>::deref>
 50e2edf:	48 89 84 24 40 02 00 	mov    %rax,0x240(%rsp)
 50e2ee6:	00 
 50e2ee7:	eb 00                	jmp    50e2ee9 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x949>
 50e2ee9:	48 8b bc 24 40 02 00 	mov    0x240(%rsp),%rdi
 50e2ef0:	00 
 50e2ef1:	e8 ea bc e3 fd       	call   2f1ebe0 <<tcl_compiler::command_binding::SourceCommandBindings>::source_origin>
 50e2ef6:	48 89 84 24 30 02 00 	mov    %rax,0x230(%rsp)
 50e2efd:	00 
 50e2efe:	eb 00                	jmp    50e2f00 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x960>
 50e2f00:	48 8b bc 24 30 02 00 	mov    0x230(%rsp),%rdi
 50e2f07:	00 
 50e2f08:	e8 f3 d3 89 ff       	call   4980300 <<core::option::Option<&alloc::sync::Arc<tcl_compiler::command_binding::SourceOriginId>> as core::ops::try_trait::Try>::branch>
 50e2f0d:	48 89 84 24 28 02 00 	mov    %rax,0x228(%rsp)
 50e2f14:	00 
 50e2f15:	eb 00                	jmp    50e2f17 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x977>
 50e2f17:	48 8b 84 24 28 02 00 	mov    0x228(%rsp),%rax
 50e2f1e:	00 
 50e2f1f:	48 89 84 24 48 0c 00 	mov    %rax,0xc48(%rsp)
 50e2f26:	00 
 50e2f27:	48 8b 94 24 48 0c 00 	mov    0xc48(%rsp),%rdx
 50e2f2e:	00 
 50e2f2f:	31 c0                	xor    %eax,%eax
 50e2f31:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e2f36:	48 83 fa 00          	cmp    $0x0,%rdx
 50e2f3a:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2f3e:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2f44:	74 12                	je     50e2f58 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x9b8>
 50e2f46:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e2f4d:	00 
 50e2f4e:	e8 8d ca 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2f53:	e9 cf 0a 00 00       	jmp    50e3a27 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1487>
 50e2f58:	48 8b 84 24 48 0c 00 	mov    0xc48(%rsp),%rax
 50e2f5f:	00 
 50e2f60:	48 89 84 24 18 02 00 	mov    %rax,0x218(%rsp)
 50e2f67:	00 
 50e2f68:	48 8d bc 24 58 04 00 	lea    0x458(%rsp),%rdi
 50e2f6f:	00 
 50e2f70:	e8 2b dc 70 ff       	call   47f0ba0 <<core::option::Option<tcl_compiler::command_binding::SourceInvocationBinding>>::as_ref>
 50e2f75:	48 89 84 24 20 02 00 	mov    %rax,0x220(%rsp)
 50e2f7c:	00 
 50e2f7d:	eb 00                	jmp    50e2f7f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x9df>
 50e2f7f:	48 8b bc 24 20 02 00 	mov    0x220(%rsp),%rdi
 50e2f86:	00 
 50e2f87:	e8 84 00 8a ff       	call   4983010 <<core::option::Option<&tcl_compiler::command_binding::SourceInvocationBinding> as core::ops::try_trait::Try>::branch>
 50e2f8c:	48 89 84 24 10 02 00 	mov    %rax,0x210(%rsp)
 50e2f93:	00 
 50e2f94:	eb 00                	jmp    50e2f96 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x9f6>
 50e2f96:	48 8b 84 24 10 02 00 	mov    0x210(%rsp),%rax
 50e2f9d:	00 
 50e2f9e:	48 89 84 24 50 0c 00 	mov    %rax,0xc50(%rsp)
 50e2fa5:	00 
 50e2fa6:	48 8b 94 24 50 0c 00 	mov    0xc50(%rsp),%rdx
 50e2fad:	00 
 50e2fae:	31 c0                	xor    %eax,%eax
 50e2fb0:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e2fb5:	48 83 fa 00          	cmp    $0x0,%rdx
 50e2fb9:	48 0f 44 c1          	cmove  %rcx,%rax
 50e2fbd:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e2fc3:	74 12                	je     50e2fd7 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xa37>
 50e2fc5:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e2fcc:	00 
 50e2fcd:	e8 0e ca 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e2fd2:	e9 4e 0a 00 00       	jmp    50e3a25 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1485>
 50e2fd7:	48 8b bc 24 50 0c 00 	mov    0xc50(%rsp),%rdi
 50e2fde:	00 
 50e2fdf:	e8 fc e6 f7 fd       	call   30616e0 <<tcl_compiler::command_binding::SourceInvocationBinding>::invocation_site>
 50e2fe4:	48 89 84 24 08 02 00 	mov    %rax,0x208(%rsp)
 50e2feb:	00 
 50e2fec:	eb 00                	jmp    50e2fee <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xa4e>
 50e2fee:	48 8b bc 24 08 02 00 	mov    0x208(%rsp),%rdi
 50e2ff5:	00 
 50e2ff6:	e8 95 fe 89 ff       	call   4982e90 <<core::option::Option<&tcl_compiler::command_binding::CommandAllocationSite> as core::ops::try_trait::Try>::branch>
 50e2ffb:	48 89 84 24 00 02 00 	mov    %rax,0x200(%rsp)
 50e3002:	00 
 50e3003:	eb 00                	jmp    50e3005 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xa65>
 50e3005:	48 8b 84 24 00 02 00 	mov    0x200(%rsp),%rax
 50e300c:	00 
 50e300d:	48 89 84 24 58 0c 00 	mov    %rax,0xc58(%rsp)
 50e3014:	00 
 50e3015:	48 8b 94 24 58 0c 00 	mov    0xc58(%rsp),%rdx
 50e301c:	00 
 50e301d:	31 c0                	xor    %eax,%eax
 50e301f:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3024:	48 83 fa 00          	cmp    $0x0,%rdx
 50e3028:	48 0f 44 c1          	cmove  %rcx,%rax
 50e302c:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3032:	74 12                	je     50e3046 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaa6>
 50e3034:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e303b:	00 
 50e303c:	e8 9f c9 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e3041:	e9 dd 09 00 00       	jmp    50e3a23 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1483>
 50e3046:	48 8b 84 24 58 0c 00 	mov    0xc58(%rsp),%rax
 50e304d:	00 
 50e304e:	48 89 84 24 f0 01 00 	mov    %rax,0x1f0(%rsp)
 50e3055:	00 
 50e3056:	48 8d bc 24 20 0b 00 	lea    0xb20(%rsp),%rdi
 50e305d:	00 
 50e305e:	e8 bd 00 71 ff       	call   47f3120 <<core::option::Option<tcl_compiler::ir::SyntheticMarker>>::is_some>
 50e3063:	88 84 24 ff 01 00 00 	mov    %al,0x1ff(%rsp)
 50e306a:	eb 00                	jmp    50e306c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xacc>
 50e306c:	8a 84 24 ff 01 00 00 	mov    0x1ff(%rsp),%al
 50e3073:	a8 01                	test   $0x1,%al
 50e3075:	75 18                	jne    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e3077:	eb 00                	jmp    50e3079 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xad9>
 50e3079:	48 8d bc 24 58 04 00 	lea    0x458(%rsp),%rdi
 50e3080:	00 
 50e3081:	e8 da e2 27 ff       	call   4361360 <<tcl_compiler::ir::CommandTokens>::original_words_match_argv_text>
 50e3086:	88 84 24 ef 01 00 00 	mov    %al,0x1ef(%rsp)
 50e308d:	eb 14                	jmp    50e30a3 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xb03>
 50e308f:	48 8b 84 24 60 03 00 	mov    0x360(%rsp),%rax
 50e3096:	00 
 50e3097:	48 c7 00 ff ff ff ff 	movq   $0xffffffffffffffff,(%rax)
 50e309e:	e9 64 09 00 00       	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e30a3:	8a 84 24 ef 01 00 00 	mov    0x1ef(%rsp),%al
 50e30aa:	a8 01                	test   $0x1,%al
 50e30ac:	75 02                	jne    50e30b0 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xb10>
 50e30ae:	eb df                	jmp    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e30b0:	48 8b 05 c9 23 83 03 	mov    0x38323c9(%rip),%rax        # 8915480 <_DYNAMIC+0x24c0>
 50e30b7:	48 8d bc 24 50 0b 00 	lea    0xb50(%rsp),%rdi
 50e30be:	00 
 50e30bf:	ff d0                	call   *%rax
 50e30c1:	48 89 94 24 d8 01 00 	mov    %rdx,0x1d8(%rsp)
 50e30c8:	00 
 50e30c9:	48 89 84 24 e0 01 00 	mov    %rax,0x1e0(%rsp)
 50e30d0:	00 
 50e30d1:	eb 00                	jmp    50e30d3 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xb33>
 50e30d3:	48 8b b4 24 d8 01 00 	mov    0x1d8(%rsp),%rsi
 50e30da:	00 
 50e30db:	48 8b bc 24 e0 01 00 	mov    0x1e0(%rsp),%rdi
 50e30e2:	00 
 50e30e3:	48 8b 05 fe 23 83 03 	mov    0x38323fe(%rip),%rax        # 89154e8 <_DYNAMIC+0x2528>
 50e30ea:	ff d0                	call   *%rax
 50e30ec:	48 89 94 24 c8 01 00 	mov    %rdx,0x1c8(%rsp)
 50e30f3:	00 
 50e30f4:	48 89 84 24 d0 01 00 	mov    %rax,0x1d0(%rsp)
 50e30fb:	00 
 50e30fc:	eb 00                	jmp    50e30fe <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xb5e>
 50e30fe:	48 8b 84 24 c8 01 00 	mov    0x1c8(%rsp),%rax
 50e3105:	00 
 50e3106:	48 8b 8c 24 d0 01 00 	mov    0x1d0(%rsp),%rcx
 50e310d:	00 
 50e310e:	48 89 8c 24 60 0c 00 	mov    %rcx,0xc60(%rsp)
 50e3115:	00 
 50e3116:	48 89 84 24 68 0c 00 	mov    %rax,0xc68(%rsp)
 50e311d:	00 
 50e311e:	48 8d bc 24 60 0c 00 	lea    0xc60(%rsp),%rdi
 50e3125:	00 
 50e3126:	e8 45 16 3c fd       	call   24a4770 <<core::slice::iter::Iter<tcl_lexer::native_word::NativeWord> as core::iter::traits::iterator::Iterator>::any::<<tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#2}>>
 50e312b:	88 84 24 c7 01 00 00 	mov    %al,0x1c7(%rsp)
 50e3132:	eb 00                	jmp    50e3134 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xb94>
 50e3134:	8a 84 24 c7 01 00 00 	mov    0x1c7(%rsp),%al
 50e313b:	a8 01                	test   $0x1,%al
 50e313d:	0f 85 4c ff ff ff    	jne    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e3143:	eb 00                	jmp    50e3145 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xba5>
 50e3145:	48 8b 84 24 50 03 00 	mov    0x350(%rsp),%rax
 50e314c:	00 
 50e314d:	48 8b 8c 24 f0 01 00 	mov    0x1f0(%rsp),%rcx
 50e3154:	00 
 50e3155:	8b 49 08             	mov    0x8(%rcx),%ecx
 50e3158:	89 8c 24 bc 01 00 00 	mov    %ecx,0x1bc(%rsp)
 50e315f:	8b b8 a8 00 00 00    	mov    0xa8(%rax),%edi
 50e3165:	8b b0 ac 00 00 00    	mov    0xac(%rax),%esi
 50e316b:	48 8b 05 e6 1c 83 03 	mov    0x3831ce6(%rip),%rax        # 8914e58 <_DYNAMIC+0x1e98>
 50e3172:	ff d0                	call   *%rax
 50e3174:	89 84 24 c0 01 00 00 	mov    %eax,0x1c0(%rsp)
 50e317b:	eb 00                	jmp    50e317d <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xbdd>
 50e317d:	8b 84 24 bc 01 00 00 	mov    0x1bc(%rsp),%eax
 50e3184:	8b 8c 24 c0 01 00 00 	mov    0x1c0(%rsp),%ecx
 50e318b:	39 c8                	cmp    %ecx,%eax
 50e318d:	0f 85 fc fe ff ff    	jne    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e3193:	48 8b b4 24 18 02 00 	mov    0x218(%rsp),%rsi
 50e319a:	00 
 50e319b:	48 8b bc 24 f0 01 00 	mov    0x1f0(%rsp),%rdi
 50e31a2:	00 
 50e31a3:	e8 48 3a f6 fe       	call   4046bf0 <<alloc::sync::Arc<tcl_compiler::command_binding::SourceOriginId>>::ptr_eq>
 50e31a8:	88 84 24 bb 01 00 00 	mov    %al,0x1bb(%rsp)
 50e31af:	eb 00                	jmp    50e31b1 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xc11>
 50e31b1:	8a 84 24 bb 01 00 00 	mov    0x1bb(%rsp),%al
 50e31b8:	a8 01                	test   $0x1,%al
 50e31ba:	75 05                	jne    50e31c1 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xc21>
 50e31bc:	e9 ce fe ff ff       	jmp    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e31c1:	48 8b bc 24 38 02 00 	mov    0x238(%rsp),%rdi
 50e31c8:	00 
 50e31c9:	e8 42 8c 06 ff       	call   414be10 <<alloc::sync::Arc<tcl_compiler::command_binding::OriginalSourceAnalysisOwner> as core::ops::deref::Deref>::deref>
 50e31ce:	48 89 84 24 b0 01 00 	mov    %rax,0x1b0(%rsp)
 50e31d5:	00 
 50e31d6:	eb 00                	jmp    50e31d8 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xc38>
 50e31d8:	48 8b bc 24 40 03 00 	mov    0x340(%rsp),%rdi
 50e31df:	00 
 50e31e0:	e8 8b 0f de ff       	call   4ec4170 <<tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory as core::ops::deref::Deref>::deref>
 50e31e5:	48 89 84 24 a8 01 00 	mov    %rax,0x1a8(%rsp)
 50e31ec:	00 
 50e31ed:	eb 00                	jmp    50e31ef <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xc4f>
 50e31ef:	48 8b 94 24 58 02 00 	mov    0x258(%rsp),%rdx
 50e31f6:	00 
 50e31f7:	48 8b b4 24 a8 01 00 	mov    0x1a8(%rsp),%rsi
 50e31fe:	00 
 50e31ff:	48 8b bc 24 b0 01 00 	mov    0x1b0(%rsp),%rdi
 50e3206:	00 
 50e3207:	e8 e4 dd f2 fd       	call   3010ff0 <<tcl_compiler::command_binding::OriginalSourceAnalysisOwner>::matches>
 50e320c:	88 84 24 a7 01 00 00 	mov    %al,0x1a7(%rsp)
 50e3213:	eb 00                	jmp    50e3215 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xc75>
 50e3215:	8a 84 24 a7 01 00 00 	mov    0x1a7(%rsp),%al
 50e321c:	a8 01                	test   $0x1,%al
 50e321e:	75 05                	jne    50e3225 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xc85>
 50e3220:	e9 6a fe ff ff       	jmp    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e3225:	48 8b bc 24 40 03 00 	mov    0x340(%rsp),%rdi
 50e322c:	00 
 50e322d:	e8 3e 0f de ff       	call   4ec4170 <<tcl_compiler::command_binding::retained_body_inventory::RetainedSourceBodyInventory as core::ops::deref::Deref>::deref>
 50e3232:	48 89 84 24 98 01 00 	mov    %rax,0x198(%rsp)
 50e3239:	00 
 50e323a:	eb 00                	jmp    50e323c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xc9c>
 50e323c:	48 8b bc 24 98 01 00 	mov    0x198(%rsp),%rdi
 50e3243:	00 
 50e3244:	e8 97 b9 e3 fd       	call   2f1ebe0 <<tcl_compiler::command_binding::SourceCommandBindings>::source_origin>
 50e3249:	48 89 84 24 90 01 00 	mov    %rax,0x190(%rsp)
 50e3250:	00 
 50e3251:	eb 00                	jmp    50e3253 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xcb3>
 50e3253:	48 8b bc 24 90 01 00 	mov    0x190(%rsp),%rdi
 50e325a:	00 
 50e325b:	e8 a0 d0 89 ff       	call   4980300 <<core::option::Option<&alloc::sync::Arc<tcl_compiler::command_binding::SourceOriginId>> as core::ops::try_trait::Try>::branch>
 50e3260:	48 89 84 24 88 01 00 	mov    %rax,0x188(%rsp)
 50e3267:	00 
 50e3268:	eb 00                	jmp    50e326a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xcca>
 50e326a:	48 8b 84 24 88 01 00 	mov    0x188(%rsp),%rax
 50e3271:	00 
 50e3272:	48 89 84 24 78 0c 00 	mov    %rax,0xc78(%rsp)
 50e3279:	00 
 50e327a:	48 8b 94 24 78 0c 00 	mov    0xc78(%rsp),%rdx
 50e3281:	00 
 50e3282:	31 c0                	xor    %eax,%eax
 50e3284:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3289:	48 83 fa 00          	cmp    $0x0,%rdx
 50e328d:	48 0f 44 c1          	cmove  %rcx,%rax
 50e3291:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3297:	74 12                	je     50e32ab <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xd0b>
 50e3299:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e32a0:	00 
 50e32a1:	e8 3a c7 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e32a6:	e9 76 07 00 00       	jmp    50e3a21 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1481>
 50e32ab:	48 8b bc 24 78 0c 00 	mov    0xc78(%rsp),%rdi
 50e32b2:	00 
 50e32b3:	e8 00 8b 06 ff       	call   414bdb8 <<alloc::sync::Arc<tcl_compiler::command_binding::SourceOriginId> as core::ops::deref::Deref>::deref>
 50e32b8:	48 89 84 24 80 01 00 	mov    %rax,0x180(%rsp)
 50e32bf:	00 
 50e32c0:	eb 00                	jmp    50e32c2 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xd22>
 50e32c2:	48 8b bc 24 80 01 00 	mov    0x180(%rsp),%rdi
 50e32c9:	00 
 50e32ca:	e8 f1 e4 e7 fd       	call   2f617c0 <<tcl_compiler::command_binding::SourceOriginId>::source_image>
 50e32cf:	48 89 84 24 78 01 00 	mov    %rax,0x178(%rsp)
 50e32d6:	00 
 50e32d7:	eb 00                	jmp    50e32d9 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xd39>
 50e32d9:	48 8b 84 24 78 01 00 	mov    0x178(%rsp),%rax
 50e32e0:	00 
 50e32e1:	48 89 84 24 70 0c 00 	mov    %rax,0xc70(%rsp)
 50e32e8:	00 
 50e32e9:	48 8b 05 98 24 83 03 	mov    0x3832498(%rip),%rax        # 8915788 <_DYNAMIC+0x27c8>
 50e32f0:	48 8d bc 24 70 0c 00 	lea    0xc70(%rsp),%rdi
 50e32f7:	00 
 50e32f8:	48 8d b4 24 80 03 00 	lea    0x380(%rsp),%rsi
 50e32ff:	00 
 50e3300:	ff d0                	call   *%rax
 50e3302:	88 84 24 77 01 00 00 	mov    %al,0x177(%rsp)
 50e3309:	eb 00                	jmp    50e330b <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xd6b>
 50e330b:	8a 84 24 77 01 00 00 	mov    0x177(%rsp),%al
 50e3312:	a8 01                	test   $0x1,%al
 50e3314:	0f 85 75 fd ff ff    	jne    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e331a:	eb 00                	jmp    50e331c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xd7c>
 50e331c:	48 8b b4 24 58 02 00 	mov    0x258(%rsp),%rsi
 50e3323:	00 
 50e3324:	48 8d bc 24 80 0c 00 	lea    0xc80(%rsp),%rdi
 50e332b:	00 
 50e332c:	e8 7f b6 eb fe       	call   3f9e9b0 <<tcl_compiler::analyser::input::ResolvedAnalysisInput>::lexer_config>
 50e3331:	eb 00                	jmp    50e3333 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xd93>
 50e3333:	48 8b b4 24 58 03 00 	mov    0x358(%rsp),%rsi
 50e333a:	00 
 50e333b:	48 81 c6 c8 09 00 00 	add    $0x9c8,%rsi
 50e3342:	48 8b 05 17 23 83 03 	mov    0x3832317(%rip),%rax        # 8915660 <_DYNAMIC+0x26a0>
 50e3349:	48 8d bc 24 80 0c 00 	lea    0xc80(%rsp),%rdi
 50e3350:	00 
 50e3351:	ff d0                	call   *%rax
 50e3353:	88 84 24 76 01 00 00 	mov    %al,0x176(%rsp)
 50e335a:	eb 00                	jmp    50e335c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xdbc>
 50e335c:	8a 84 24 76 01 00 00 	mov    0x176(%rsp),%al
 50e3363:	a8 01                	test   $0x1,%al
 50e3365:	0f 85 24 fd ff ff    	jne    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e336b:	eb 00                	jmp    50e336d <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xdcd>
 50e336d:	48 8b 84 24 58 03 00 	mov    0x358(%rsp),%rax
 50e3374:	00 
 50e3375:	48 8b b0 e8 08 00 00 	mov    0x8e8(%rax),%rsi
 50e337c:	48 8b 84 24 30 0c 00 	mov    0xc30(%rsp),%rax
 50e3383:	00 
 50e3384:	48 89 84 24 c0 13 00 	mov    %rax,0x13c0(%rsp)
 50e338b:	00 
 50e338c:	0f 28 84 24 20 0c 00 	movaps 0xc20(%rsp),%xmm0
 50e3393:	00 
 50e3394:	0f 29 84 24 b0 13 00 	movaps %xmm0,0x13b0(%rsp)
 50e339b:	00 
 50e339c:	48 8d bc 24 b0 13 00 	lea    0x13b0(%rsp),%rdi
 50e33a3:	00 
 50e33a4:	e8 47 86 48 fe       	call   356b9f0 <<tcl_compiler::registry_invocation::metadata_context::InvocationMetadataContext>::matches_registry>
 50e33a9:	88 84 24 75 01 00 00 	mov    %al,0x175(%rsp)
 50e33b0:	eb 00                	jmp    50e33b2 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xe12>
 50e33b2:	8a 84 24 75 01 00 00 	mov    0x175(%rsp),%al
 50e33b9:	a8 01                	test   $0x1,%al
 50e33bb:	75 05                	jne    50e33c2 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xe22>
 50e33bd:	e9 cd fc ff ff       	jmp    50e308f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xaef>
 50e33c2:	48 8b bc 24 58 02 00 	mov    0x258(%rsp),%rdi
 50e33c9:	00 
 50e33ca:	e8 19 b7 eb fe       	call   3f9eae8 <<tcl_compiler::analyser::input::ResolvedAnalysisInput>::vendor_source_policy>
 50e33cf:	88 94 24 73 01 00 00 	mov    %dl,0x173(%rsp)
 50e33d6:	88 84 24 74 01 00 00 	mov    %al,0x174(%rsp)
 50e33dd:	eb 00                	jmp    50e33df <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xe3f>
 50e33df:	48 8b 94 24 58 02 00 	mov    0x258(%rsp),%rdx
 50e33e6:	00 
 50e33e7:	8a 84 24 73 01 00 00 	mov    0x173(%rsp),%al
 50e33ee:	8a 8c 24 74 01 00 00 	mov    0x174(%rsp),%cl
 50e33f5:	0f b6 f9             	movzbl %cl,%edi
 50e33f8:	0f b6 f0             	movzbl %al,%esi
 50e33fb:	e8 50 44 58 ff       	call   4667850 <<core::option::Option<tcl_syntax::naming::vendor_source::VendorSourceNamePolicy>>::or_else::<<tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#3}>>
 50e3400:	88 94 24 71 01 00 00 	mov    %dl,0x171(%rsp)
 50e3407:	88 84 24 72 01 00 00 	mov    %al,0x172(%rsp)
 50e340e:	eb 00                	jmp    50e3410 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xe70>
 50e3410:	8a 84 24 71 01 00 00 	mov    0x171(%rsp),%al
 50e3417:	8a 8c 24 72 01 00 00 	mov    0x172(%rsp),%cl
 50e341e:	0f b6 f9             	movzbl %cl,%edi
 50e3421:	0f b6 f0             	movzbl %al,%esi
 50e3424:	48 8b 05 5d 5c 83 03 	mov    0x3835c5d(%rip),%rax        # 8919088 <_DYNAMIC+0x60c8>
 50e342b:	ff d0                	call   *%rax
 50e342d:	88 94 24 6f 01 00 00 	mov    %dl,0x16f(%rsp)
 50e3434:	88 84 24 70 01 00 00 	mov    %al,0x170(%rsp)
 50e343b:	eb 00                	jmp    50e343d <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xe9d>
 50e343d:	8a 84 24 6f 01 00 00 	mov    0x16f(%rsp),%al
 50e3444:	8a 8c 24 70 01 00 00 	mov    0x170(%rsp),%cl
 50e344b:	88 8c 24 9e 0c 00 00 	mov    %cl,0xc9e(%rsp)
 50e3452:	88 84 24 9f 0c 00 00 	mov    %al,0xc9f(%rsp)
 50e3459:	31 c0                	xor    %eax,%eax
 50e345b:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3460:	80 bc 24 9e 0c 00 00 	cmpb   $0x2,0xc9e(%rsp)
 50e3467:	02 
 50e3468:	48 0f 44 c1          	cmove  %rcx,%rax
 50e346c:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3472:	74 12                	je     50e3486 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xee6>
 50e3474:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e347b:	00 
 50e347c:	e8 5f c5 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e3481:	e9 99 05 00 00       	jmp    50e3a1f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x147f>
 50e3486:	8a 84 24 9e 0c 00 00 	mov    0xc9e(%rsp),%al
 50e348d:	88 84 24 56 01 00 00 	mov    %al,0x156(%rsp)
 50e3494:	8a 84 24 9f 0c 00 00 	mov    0xc9f(%rsp),%al
 50e349b:	88 84 24 57 01 00 00 	mov    %al,0x157(%rsp)
 50e34a2:	48 8b 05 d7 1f 83 03 	mov    0x3831fd7(%rip),%rax        # 8915480 <_DYNAMIC+0x24c0>
 50e34a9:	48 8d bc 24 50 0b 00 	lea    0xb50(%rsp),%rdi
 50e34b0:	00 
 50e34b1:	ff d0                	call   *%rax
 50e34b3:	48 89 94 24 58 01 00 	mov    %rdx,0x158(%rsp)
 50e34ba:	00 
 50e34bb:	48 89 84 24 60 01 00 	mov    %rax,0x160(%rsp)
 50e34c2:	00 
 50e34c3:	eb 00                	jmp    50e34c5 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xf25>
 50e34c5:	48 8b b4 24 58 01 00 	mov    0x158(%rsp),%rsi
 50e34cc:	00 
 50e34cd:	48 8b bc 24 60 01 00 	mov    0x160(%rsp),%rdi
 50e34d4:	00 
 50e34d5:	48 8b 05 6c 14 83 03 	mov    0x383146c(%rip),%rax        # 8914948 <_DYNAMIC+0x1988>
 50e34dc:	ff d0                	call   *%rax
 50e34de:	48 89 84 24 48 01 00 	mov    %rax,0x148(%rsp)
 50e34e5:	00 
 50e34e6:	eb 00                	jmp    50e34e8 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xf48>
 50e34e8:	48 8b bc 24 48 01 00 	mov    0x148(%rsp),%rdi
 50e34ef:	00 
 50e34f0:	48 8b 05 59 14 83 03 	mov    0x3831459(%rip),%rax        # 8914950 <_DYNAMIC+0x1990>
 50e34f7:	ff d0                	call   *%rax
 50e34f9:	48 89 84 24 40 01 00 	mov    %rax,0x140(%rsp)
 50e3500:	00 
 50e3501:	eb 00                	jmp    50e3503 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xf63>
 50e3503:	48 8b 84 24 40 01 00 	mov    0x140(%rsp),%rax
 50e350a:	00 
 50e350b:	48 89 84 24 68 0d 00 	mov    %rax,0xd68(%rsp)
 50e3512:	00 
 50e3513:	48 8b 94 24 68 0d 00 	mov    0xd68(%rsp),%rdx
 50e351a:	00 
 50e351b:	31 c0                	xor    %eax,%eax
 50e351d:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3522:	48 83 fa 00          	cmp    $0x0,%rdx
 50e3526:	48 0f 44 c1          	cmove  %rcx,%rax
 50e352a:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3530:	74 12                	je     50e3544 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xfa4>
 50e3532:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e3539:	00 
 50e353a:	e8 a1 c4 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e353f:	e9 d9 04 00 00       	jmp    50e3a1d <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x147d>
 50e3544:	8a 84 24 56 01 00 00 	mov    0x156(%rsp),%al
 50e354b:	8a 8c 24 57 01 00 00 	mov    0x157(%rsp),%cl
 50e3552:	48 8b b4 24 68 0d 00 	mov    0xd68(%rsp),%rsi
 50e3559:	00 
 50e355a:	0f b6 c9             	movzbl %cl,%ecx
 50e355d:	0f b6 d0             	movzbl %al,%edx
 50e3560:	83 e2 01             	and    $0x1,%edx
 50e3563:	48 8d bc 24 a0 0c 00 	lea    0xca0(%rsp),%rdi
 50e356a:	00 
 50e356b:	e8 70 b1 eb fe       	call   3f9e6e0 <<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>::from_original_word>
 50e3570:	eb 00                	jmp    50e3572 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xfd2>
 50e3572:	48 8d bc 24 a0 0c 00 	lea    0xca0(%rsp),%rdi
 50e3579:	00 
 50e357a:	31 f6                	xor    %esi,%esi
 50e357c:	e8 6f ad eb fe       	call   3f9e2f0 <<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>::literal_units>
 50e3581:	48 89 94 24 30 01 00 	mov    %rdx,0x130(%rsp)
 50e3588:	00 
 50e3589:	48 89 84 24 38 01 00 	mov    %rax,0x138(%rsp)
 50e3590:	00 
 50e3591:	eb 28                	jmp    50e35bb <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x101b>
 50e3593:	48 8d bc 24 a0 0c 00 	lea    0xca0(%rsp),%rdi
 50e359a:	00 
 50e359b:	e8 f0 37 eb ff       	call   4f96d90 <core::ptr::drop_glue::<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>>
 50e35a0:	e9 ec f5 ff ff       	jmp    50e2b91 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x5f1>
 50e35a5:	48 89 c1             	mov    %rax,%rcx
 50e35a8:	89 d0                	mov    %edx,%eax
 50e35aa:	48 89 8c 24 60 13 00 	mov    %rcx,0x1360(%rsp)
 50e35b1:	00 
 50e35b2:	89 84 24 68 13 00 00 	mov    %eax,0x1368(%rsp)
 50e35b9:	eb d8                	jmp    50e3593 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xff3>
 50e35bb:	48 8b b4 24 30 01 00 	mov    0x130(%rsp),%rsi
 50e35c2:	00 
 50e35c3:	48 8b bc 24 38 01 00 	mov    0x138(%rsp),%rdi
 50e35ca:	00 
 50e35cb:	48 8b 05 d6 20 83 03 	mov    0x38320d6(%rip),%rax        # 89156a8 <_DYNAMIC+0x26e8>
 50e35d2:	ff d0                	call   *%rax
 50e35d4:	48 89 94 24 20 01 00 	mov    %rdx,0x120(%rsp)
 50e35db:	00 
 50e35dc:	48 89 84 24 28 01 00 	mov    %rax,0x128(%rsp)
 50e35e3:	00 
 50e35e4:	eb 00                	jmp    50e35e6 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1046>
 50e35e6:	48 8b 84 24 20 01 00 	mov    0x120(%rsp),%rax
 50e35ed:	00 
 50e35ee:	48 8b 8c 24 28 01 00 	mov    0x128(%rsp),%rcx
 50e35f5:	00 
 50e35f6:	48 89 8c 24 98 0d 00 	mov    %rcx,0xd98(%rsp)
 50e35fd:	00 
 50e35fe:	48 89 84 24 a0 0d 00 	mov    %rax,0xda0(%rsp)
 50e3605:	00 
 50e3606:	48 8b 94 24 98 0d 00 	mov    0xd98(%rsp),%rdx
 50e360d:	00 
 50e360e:	31 c0                	xor    %eax,%eax
 50e3610:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3615:	48 83 fa 00          	cmp    $0x0,%rdx
 50e3619:	48 0f 44 c1          	cmove  %rcx,%rax
 50e361d:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3623:	74 12                	je     50e3637 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1097>
 50e3625:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e362c:	00 
 50e362d:	e8 ae c3 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e3632:	e9 ce 03 00 00       	jmp    50e3a05 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1465>
 50e3637:	48 8b b4 24 98 0d 00 	mov    0xd98(%rsp),%rsi
 50e363e:	00 
 50e363f:	48 8b 94 24 a0 0d 00 	mov    0xda0(%rsp),%rdx
 50e3646:	00 
 50e3647:	48 8b 05 9a 14 83 03 	mov    0x383149a(%rip),%rax        # 8914ae8 <_DYNAMIC+0x1b28>
 50e364e:	48 8d bc 24 80 0d 00 	lea    0xd80(%rsp),%rdi
 50e3655:	00 
 50e3656:	ff d0                	call   *%rax
 50e3658:	eb 00                	jmp    50e365a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x10ba>
 50e365a:	48 8b 05 8f 14 83 03 	mov    0x383148f(%rip),%rax        # 8914af0 <_DYNAMIC+0x1b30>
 50e3661:	48 8d bc 24 80 0d 00 	lea    0xd80(%rsp),%rdi
 50e3668:	00 
 50e3669:	ff d0                	call   *%rax
 50e366b:	48 89 94 24 10 01 00 	mov    %rdx,0x110(%rsp)
 50e3672:	00 
 50e3673:	48 89 84 24 18 01 00 	mov    %rax,0x118(%rsp)
 50e367a:	00 
 50e367b:	eb 00                	jmp    50e367d <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x10dd>
 50e367d:	48 8b b4 24 10 01 00 	mov    0x110(%rsp),%rsi
 50e3684:	00 
 50e3685:	48 8b bc 24 18 01 00 	mov    0x118(%rsp),%rdi
 50e368c:	00 
 50e368d:	48 8b 05 64 14 83 03 	mov    0x3831464(%rip),%rax        # 8914af8 <_DYNAMIC+0x1b38>
 50e3694:	ff d0                	call   *%rax
 50e3696:	48 89 94 24 00 01 00 	mov    %rdx,0x100(%rsp)
 50e369d:	00 
 50e369e:	48 89 84 24 08 01 00 	mov    %rax,0x108(%rsp)
 50e36a5:	00 
 50e36a6:	eb 00                	jmp    50e36a8 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1108>
 50e36a8:	48 8b 84 24 00 01 00 	mov    0x100(%rsp),%rax
 50e36af:	00 
 50e36b0:	48 8b 8c 24 08 01 00 	mov    0x108(%rsp),%rcx
 50e36b7:	00 
 50e36b8:	48 89 8c 24 70 0d 00 	mov    %rcx,0xd70(%rsp)
 50e36bf:	00 
 50e36c0:	48 89 84 24 78 0d 00 	mov    %rax,0xd78(%rsp)
 50e36c7:	00 
 50e36c8:	48 8b 94 24 70 0d 00 	mov    0xd70(%rsp),%rdx
 50e36cf:	00 
 50e36d0:	31 c0                	xor    %eax,%eax
 50e36d2:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e36d7:	48 83 fa 00          	cmp    $0x0,%rdx
 50e36db:	48 0f 44 c1          	cmove  %rcx,%rax
 50e36df:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e36e5:	74 12                	je     50e36f9 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1159>
 50e36e7:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e36ee:	00 
 50e36ef:	e8 ec c2 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e36f4:	e9 0a 03 00 00       	jmp    50e3a03 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1463>
 50e36f9:	48 8b bc 24 70 0d 00 	mov    0xd70(%rsp),%rdi
 50e3700:	00 
 50e3701:	48 89 bc 24 e0 00 00 	mov    %rdi,0xe0(%rsp)
 50e3708:	00 
 50e3709:	48 8b b4 24 78 0d 00 	mov    0xd78(%rsp),%rsi
 50e3710:	00 
 50e3711:	48 89 b4 24 e8 00 00 	mov    %rsi,0xe8(%rsp)
 50e3718:	00 
 50e3719:	48 8b 05 08 18 83 03 	mov    0x3831808(%rip),%rax        # 8914f28 <_DYNAMIC+0x1f68>
 50e3720:	ff d0                	call   *%rax
 50e3722:	48 89 94 24 f0 00 00 	mov    %rdx,0xf0(%rsp)
 50e3729:	00 
 50e372a:	48 89 84 24 f8 00 00 	mov    %rax,0xf8(%rsp)
 50e3731:	00 
 50e3732:	eb 00                	jmp    50e3734 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1194>
 50e3734:	48 8b 8c 24 e8 00 00 	mov    0xe8(%rsp),%rcx
 50e373b:	00 
 50e373c:	48 8b 94 24 e0 00 00 	mov    0xe0(%rsp),%rdx
 50e3743:	00 
 50e3744:	48 8b b4 24 f0 00 00 	mov    0xf0(%rsp),%rsi
 50e374b:	00 
 50e374c:	48 8b bc 24 f8 00 00 	mov    0xf8(%rsp),%rdi
 50e3753:	00 
 50e3754:	48 8b 05 d5 17 83 03 	mov    0x38317d5(%rip),%rax        # 8914f30 <_DYNAMIC+0x1f70>
 50e375b:	ff d0                	call   *%rax
 50e375d:	48 89 94 24 d0 00 00 	mov    %rdx,0xd0(%rsp)
 50e3764:	00 
 50e3765:	48 89 84 24 d8 00 00 	mov    %rax,0xd8(%rsp)
 50e376c:	00 
 50e376d:	eb 00                	jmp    50e376f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x11cf>
 50e376f:	48 8b b4 24 38 03 00 	mov    0x338(%rsp),%rsi
 50e3776:	00 
 50e3777:	48 8b bc 24 30 03 00 	mov    0x330(%rsp),%rdi
 50e377e:	00 
 50e377f:	48 8b 84 24 d0 00 00 	mov    0xd0(%rsp),%rax
 50e3786:	00 
 50e3787:	48 8b 8c 24 d8 00 00 	mov    0xd8(%rsp),%rcx
 50e378e:	00 
 50e378f:	48 89 8c 24 a8 0d 00 	mov    %rcx,0xda8(%rsp)
 50e3796:	00 
 50e3797:	48 89 84 24 b0 0d 00 	mov    %rax,0xdb0(%rsp)
 50e379e:	00 
 50e379f:	e8 14 09 52 00       	call   56040b8 <<[tcl_compiler::command_binding::rule_declaration::SourceRuleDeclarationCandidate]>::iter>
 50e37a4:	48 89 94 24 c0 00 00 	mov    %rdx,0xc0(%rsp)
 50e37ab:	00 
 50e37ac:	48 89 84 24 c8 00 00 	mov    %rax,0xc8(%rsp)
 50e37b3:	00 
 50e37b4:	eb 00                	jmp    50e37b6 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1216>
 50e37b6:	48 8b 84 24 c0 00 00 	mov    0xc0(%rsp),%rax
 50e37bd:	00 
 50e37be:	48 8b 8c 24 c8 00 00 	mov    0xc8(%rsp),%rcx
 50e37c5:	00 
 50e37c6:	48 89 8c 24 b8 0d 00 	mov    %rcx,0xdb8(%rsp)
 50e37cd:	00 
 50e37ce:	48 89 84 24 c0 0d 00 	mov    %rax,0xdc0(%rsp)
 50e37d5:	00 
 50e37d6:	48 8d bc 24 b8 0d 00 	lea    0xdb8(%rsp),%rdi
 50e37dd:	00 
 50e37de:	48 8d b4 24 a8 0d 00 	lea    0xda8(%rsp),%rsi
 50e37e5:	00 
 50e37e6:	e8 a5 7a 47 fd       	call   255b290 <<core::slice::iter::Iter<tcl_compiler::command_binding::rule_declaration::SourceRuleDeclarationCandidate> as core::iter::traits::iterator::Iterator>::any::<<tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#4}>>
 50e37eb:	88 84 24 bf 00 00 00 	mov    %al,0xbf(%rsp)
 50e37f2:	eb 00                	jmp    50e37f4 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1254>
 50e37f4:	8a 84 24 bf 00 00 00 	mov    0xbf(%rsp),%al
 50e37fb:	a8 01                	test   $0x1,%al
 50e37fd:	75 30                	jne    50e382f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x128f>
 50e37ff:	eb 00                	jmp    50e3801 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1261>
 50e3801:	48 8b b4 24 88 02 00 	mov    0x288(%rsp),%rsi
 50e3808:	00 
 50e3809:	8a 84 24 56 01 00 00 	mov    0x156(%rsp),%al
 50e3810:	8a 8c 24 57 01 00 00 	mov    0x157(%rsp),%cl
 50e3817:	0f b6 c9             	movzbl %cl,%ecx
 50e381a:	0f b6 d0             	movzbl %al,%edx
 50e381d:	83 e2 01             	and    $0x1,%edx
 50e3820:	48 8d bc 24 c8 0d 00 	lea    0xdc8(%rsp),%rdi
 50e3827:	00 
 50e3828:	e8 b3 ae eb fe       	call   3f9e6e0 <<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>::from_original_word>
 50e382d:	eb 14                	jmp    50e3843 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x12a3>
 50e382f:	48 8b 84 24 60 03 00 	mov    0x360(%rsp),%rax
 50e3836:	00 
 50e3837:	48 c7 00 ff ff ff ff 	movq   $0xffffffffffffffff,(%rax)
 50e383e:	e9 ab 01 00 00       	jmp    50e39ee <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x144e>
 50e3843:	48 8d bc 24 c8 0d 00 	lea    0xdc8(%rsp),%rdi
 50e384a:	00 
 50e384b:	be 02 00 00 00       	mov    $0x2,%esi
 50e3850:	e8 9b aa eb fe       	call   3f9e2f0 <<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>::literal_units>
 50e3855:	48 89 94 24 a8 00 00 	mov    %rdx,0xa8(%rsp)
 50e385c:	00 
 50e385d:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
 50e3864:	00 
 50e3865:	eb 28                	jmp    50e388f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x12ef>
 50e3867:	48 8d bc 24 c8 0d 00 	lea    0xdc8(%rsp),%rdi
 50e386e:	00 
 50e386f:	e8 1c 35 eb ff       	call   4f96d90 <core::ptr::drop_glue::<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>>
 50e3874:	e9 1a fd ff ff       	jmp    50e3593 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0xff3>
 50e3879:	48 89 c1             	mov    %rax,%rcx
 50e387c:	89 d0                	mov    %edx,%eax
 50e387e:	48 89 8c 24 60 13 00 	mov    %rcx,0x1360(%rsp)
 50e3885:	00 
 50e3886:	89 84 24 68 13 00 00 	mov    %eax,0x1368(%rsp)
 50e388d:	eb d8                	jmp    50e3867 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x12c7>
 50e388f:	48 8b b4 24 a8 00 00 	mov    0xa8(%rsp),%rsi
 50e3896:	00 
 50e3897:	48 8b bc 24 b0 00 00 	mov    0xb0(%rsp),%rdi
 50e389e:	00 
 50e389f:	48 8d 15 21 61 d3 fb 	lea    -0x42c9edf(%rip),%rdx        # e199c7 <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153ab7>
 50e38a6:	b9 12 00 00 00       	mov    $0x12,%ecx
 50e38ab:	e8 20 36 e2 ff       	call   4f06ed0 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<&[u8]>>
 50e38b0:	48 89 94 24 98 00 00 	mov    %rdx,0x98(%rsp)
 50e38b7:	00 
 50e38b8:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
 50e38bf:	00 
 50e38c0:	eb 00                	jmp    50e38c2 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1322>
 50e38c2:	48 8b b4 24 98 00 00 	mov    0x98(%rsp),%rsi
 50e38c9:	00 
 50e38ca:	48 8b bc 24 a0 00 00 	mov    0xa0(%rsp),%rdi
 50e38d1:	00 
 50e38d2:	48 8b 05 cf 1d 83 03 	mov    0x3831dcf(%rip),%rax        # 89156a8 <_DYNAMIC+0x26e8>
 50e38d9:	ff d0                	call   *%rax
 50e38db:	48 89 94 24 88 00 00 	mov    %rdx,0x88(%rsp)
 50e38e2:	00 
 50e38e3:	48 89 84 24 90 00 00 	mov    %rax,0x90(%rsp)
 50e38ea:	00 
 50e38eb:	eb 00                	jmp    50e38ed <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x134d>
 50e38ed:	48 8b 84 24 88 00 00 	mov    0x88(%rsp),%rax
 50e38f4:	00 
 50e38f5:	48 8b 8c 24 90 00 00 	mov    0x90(%rsp),%rcx
 50e38fc:	00 
 50e38fd:	48 89 8c 24 90 0e 00 	mov    %rcx,0xe90(%rsp)
 50e3904:	00 
 50e3905:	48 89 84 24 98 0e 00 	mov    %rax,0xe98(%rsp)
 50e390c:	00 
 50e390d:	48 8b 94 24 90 0e 00 	mov    0xe90(%rsp),%rdx
 50e3914:	00 
 50e3915:	31 c0                	xor    %eax,%eax
 50e3917:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e391c:	48 83 fa 00          	cmp    $0x0,%rdx
 50e3920:	48 0f 44 c1          	cmove  %rcx,%rax
 50e3924:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e392a:	74 12                	je     50e393e <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x139e>
 50e392c:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e3933:	00 
 50e3934:	e8 a7 c0 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e3939:	e9 ae 00 00 00       	jmp    50e39ec <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x144c>
 50e393e:	48 8b 84 24 90 0e 00 	mov    0xe90(%rsp),%rax
 50e3945:	00 
 50e3946:	48 89 44 24 68       	mov    %rax,0x68(%rsp)
 50e394b:	48 8b 84 24 98 0e 00 	mov    0xe98(%rsp),%rax
 50e3952:	00 
 50e3953:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
 50e3958:	48 8b 84 24 70 03 00 	mov    0x370(%rsp),%rax
 50e395f:	00 
 50e3960:	48 89 44 24 78       	mov    %rax,0x78(%rsp)
 50e3965:	48 8b 84 24 78 03 00 	mov    0x378(%rsp),%rax
 50e396c:	00 
 50e396d:	48 89 84 24 80 00 00 	mov    %rax,0x80(%rsp)
 50e3974:	00 
 50e3975:	48 8b 8c 24 80 00 00 	mov    0x80(%rsp),%rcx
 50e397c:	00 
 50e397d:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
 50e3982:	48 8b 74 24 70       	mov    0x70(%rsp),%rsi
 50e3987:	48 8b 7c 24 68       	mov    0x68(%rsp),%rdi
 50e398c:	e8 ef a1 b2 fd       	call   2c0db80 <<[u8]>::eq_ignore_ascii_case>
 50e3991:	88 44 24 67          	mov    %al,0x67(%rsp)
 50e3995:	eb 00                	jmp    50e3997 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x13f7>
 50e3997:	48 8b 94 24 88 02 00 	mov    0x288(%rsp),%rdx
 50e399e:	00 
 50e399f:	8a 44 24 67          	mov    0x67(%rsp),%al
 50e39a3:	0f b6 f0             	movzbl %al,%esi
 50e39a6:	83 e6 01             	and    $0x1,%esi
 50e39a9:	48 8d bc 24 a0 0e 00 	lea    0xea0(%rsp),%rdi
 50e39b0:	00 
 50e39b1:	e8 6a b3 e0 ff       	call   4eeed20 <<bool>::then::<tcl_lexer::native_word::NativeWord, <tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#5}>>
 50e39b6:	eb 00                	jmp    50e39b8 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1418>
 50e39b8:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e39bf:	00 
 50e39c0:	48 8d 15 12 60 d3 fb 	lea    -0x42c9fee(%rip),%rdx        # e199d9 <tcl_compiler::ir::NO_DEFERRED_WRITES+0x153ac9>
 50e39c7:	48 8d b4 24 a0 0e 00 	lea    0xea0(%rsp),%rsi
 50e39ce:	00 
 50e39cf:	b9 12 00 00 00       	mov    $0x12,%ecx
 50e39d4:	e8 e7 2d e2 ff       	call   4f067c0 <<tcl_compiler::lowering::Lowerer>::trace_report_event_option::<tcl_lexer::native_word::NativeWord>>
 50e39d9:	eb 00                	jmp    50e39db <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x143b>
 50e39db:	eb 00                	jmp    50e39dd <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x143d>
 50e39dd:	48 8d bc 24 c8 0d 00 	lea    0xdc8(%rsp),%rdi
 50e39e4:	00 
 50e39e5:	e8 a6 33 eb ff       	call   4f96d90 <core::ptr::drop_glue::<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>>
 50e39ea:	eb 02                	jmp    50e39ee <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x144e>
 50e39ec:	eb ef                	jmp    50e39dd <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x143d>
 50e39ee:	48 8d bc 24 a0 0c 00 	lea    0xca0(%rsp),%rdi
 50e39f5:	00 
 50e39f6:	e8 95 33 eb ff       	call   4f96d90 <core::ptr::drop_glue::<tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput>>
 50e39fb:	eb 0a                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e39fd:	ff 15 0d f8 82 03    	call   *0x382f80d(%rip)        # 8913210 <_DYNAMIC+0x250>
 50e3a03:	eb e9                	jmp    50e39ee <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x144e>
 50e3a05:	eb e7                	jmp    50e39ee <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x144e>
 50e3a07:	48 8b 05 a2 08 83 03 	mov    0x38308a2(%rip),%rax        # 89142b0 <_DYNAMIC+0x12f0>
 50e3a0e:	48 8d bc 24 50 0b 00 	lea    0xb50(%rsp),%rdi
 50e3a15:	00 
 50e3a16:	ff d0                	call   *%rax
 50e3a18:	e9 0e 03 00 00       	jmp    50e3d2b <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x178b>
 50e3a1d:	eb e8                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a1f:	eb e6                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a21:	eb e4                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a23:	eb e2                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a25:	eb e0                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a27:	eb de                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a29:	eb dc                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a2b:	eb da                	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3a2d:	31 c0                	xor    %eax,%eax
 50e3a2f:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3a34:	80 bc 24 88 0f 00 00 	cmpb   $0x2,0xf88(%rsp)
 50e3a3b:	02 
 50e3a3c:	48 0f 44 c1          	cmove  %rcx,%rax
 50e3a40:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3a46:	74 12                	je     50e3a5a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x14ba>
 50e3a48:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e3a4f:	00 
 50e3a50:	e8 8b bf 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e3a55:	e9 c7 02 00 00       	jmp    50e3d21 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1781>
 50e3a5a:	0f 10 84 24 60 0f 00 	movups 0xf60(%rsp),%xmm0
 50e3a61:	00 
 50e3a62:	0f 10 8c 24 70 0f 00 	movups 0xf70(%rsp),%xmm1
 50e3a69:	00 
 50e3a6a:	0f 10 94 24 80 0f 00 	movups 0xf80(%rsp),%xmm2
 50e3a71:	00 
 50e3a72:	0f 29 94 24 e0 0f 00 	movaps %xmm2,0xfe0(%rsp)
 50e3a79:	00 
 50e3a7a:	0f 29 8c 24 d0 0f 00 	movaps %xmm1,0xfd0(%rsp)
 50e3a81:	00 
 50e3a82:	0f 29 84 24 c0 0f 00 	movaps %xmm0,0xfc0(%rsp)
 50e3a89:	00 
 50e3a8a:	8a 84 24 e8 0f 00 00 	mov    0xfe8(%rsp),%al
 50e3a91:	88 44 24 63          	mov    %al,0x63(%rsp)
 50e3a95:	8a 84 24 e9 0f 00 00 	mov    0xfe9(%rsp),%al
 50e3a9c:	88 44 24 64          	mov    %al,0x64(%rsp)
 50e3aa0:	0f 28 84 24 c0 0f 00 	movaps 0xfc0(%rsp),%xmm0
 50e3aa7:	00 
 50e3aa8:	0f 28 8c 24 d0 0f 00 	movaps 0xfd0(%rsp),%xmm1
 50e3aaf:	00 
 50e3ab0:	0f 28 94 24 e0 0f 00 	movaps 0xfe0(%rsp),%xmm2
 50e3ab7:	00 
 50e3ab8:	0f 29 94 24 f0 13 00 	movaps %xmm2,0x13f0(%rsp)
 50e3abf:	00 
 50e3ac0:	0f 29 8c 24 e0 13 00 	movaps %xmm1,0x13e0(%rsp)
 50e3ac7:	00 
 50e3ac8:	0f 29 84 24 d0 13 00 	movaps %xmm0,0x13d0(%rsp)
 50e3acf:	00 
 50e3ad0:	48 8b 05 d9 16 83 03 	mov    0x38316d9(%rip),%rax        # 89151b0 <_DYNAMIC+0x21f0>
 50e3ad7:	48 8d bc 24 d0 13 00 	lea    0x13d0(%rsp),%rdi
 50e3ade:	00 
 50e3adf:	ff d0                	call   *%rax
 50e3ae1:	88 54 24 65          	mov    %dl,0x65(%rsp)
 50e3ae5:	88 44 24 66          	mov    %al,0x66(%rsp)
 50e3ae9:	eb 00                	jmp    50e3aeb <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x154b>
 50e3aeb:	8a 44 24 65          	mov    0x65(%rsp),%al
 50e3aef:	8a 4c 24 66          	mov    0x66(%rsp),%cl
 50e3af3:	0f b6 f9             	movzbl %cl,%edi
 50e3af6:	0f b6 f0             	movzbl %al,%esi
 50e3af9:	48 8b 05 28 1d 83 03 	mov    0x3831d28(%rip),%rax        # 8915828 <_DYNAMIC+0x2868>
 50e3b00:	ff d0                	call   *%rax
 50e3b02:	88 54 24 61          	mov    %dl,0x61(%rsp)
 50e3b06:	88 44 24 62          	mov    %al,0x62(%rsp)
 50e3b0a:	eb 00                	jmp    50e3b0c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x156c>
 50e3b0c:	8a 44 24 61          	mov    0x61(%rsp),%al
 50e3b10:	8a 4c 24 62          	mov    0x62(%rsp),%cl
 50e3b14:	88 8c 24 86 12 00 00 	mov    %cl,0x1286(%rsp)
 50e3b1b:	88 84 24 87 12 00 00 	mov    %al,0x1287(%rsp)
 50e3b22:	31 c0                	xor    %eax,%eax
 50e3b24:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3b29:	80 bc 24 86 12 00 00 	cmpb   $0x2,0x1286(%rsp)
 50e3b30:	02 
 50e3b31:	48 0f 44 c1          	cmove  %rcx,%rax
 50e3b35:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3b3b:	74 12                	je     50e3b4f <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x15af>
 50e3b3d:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e3b44:	00 
 50e3b45:	e8 96 be 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e3b4a:	e9 cd 01 00 00       	jmp    50e3d1c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x177c>
 50e3b4f:	48 8b b4 24 88 02 00 	mov    0x288(%rsp),%rsi
 50e3b56:	00 
 50e3b57:	8a 44 24 64          	mov    0x64(%rsp),%al
 50e3b5b:	8a 4c 24 63          	mov    0x63(%rsp),%cl
 50e3b5f:	44 0f b6 84 24 86 12 	movzbl 0x1286(%rsp),%r8d
 50e3b66:	00 00 
 50e3b68:	44 0f b6 8c 24 87 12 	movzbl 0x1287(%rsp),%r9d
 50e3b6f:	00 00 
 50e3b71:	0f b6 d1             	movzbl %cl,%edx
 50e3b74:	83 e2 01             	and    $0x1,%edx
 50e3b77:	0f b6 c8             	movzbl %al,%ecx
 50e3b7a:	83 e1 01             	and    $0x1,%ecx
 50e3b7d:	48 8d bc 24 a8 11 00 	lea    0x11a8(%rsp),%rdi
 50e3b84:	00 
 50e3b85:	e8 36 d2 9b fe       	call   3aa0dc0 <<tcl_compiler::signature_scan::original_name::SignatureSourceNameKey>::from_original_native_word>
 50e3b8a:	eb 00                	jmp    50e3b8c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x15ec>
 50e3b8c:	48 8d bc 24 d0 10 00 	lea    0x10d0(%rsp),%rdi
 50e3b93:	00 
 50e3b94:	48 8d b4 24 a8 11 00 	lea    0x11a8(%rsp),%rsi
 50e3b9b:	00 
 50e3b9c:	e8 3f 34 89 ff       	call   4976fe0 <<core::option::Option<tcl_compiler::signature_scan::original_name::SignatureSourceNameKey> as core::ops::try_trait::Try>::branch>
 50e3ba1:	eb 00                	jmp    50e3ba3 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1603>
 50e3ba3:	31 c0                	xor    %eax,%eax
 50e3ba5:	b9 01 00 00 00       	mov    $0x1,%ecx
 50e3baa:	48 83 bc 24 d0 10 00 	cmpq   $0xffffffffffffffff,0x10d0(%rsp)
 50e3bb1:	00 ff 
 50e3bb3:	48 0f 44 c1          	cmove  %rcx,%rax
 50e3bb7:	48 a9 01 00 00 00    	test   $0x1,%rax
 50e3bbd:	74 12                	je     50e3bd1 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1631>
 50e3bbf:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e3bc6:	00 
 50e3bc7:	e8 14 be 8a ff       	call   498f9e0 <<core::option::Option<tcl_lexer::native_word::NativeWord> as core::ops::try_trait::FromResidual<core::option::Option<core::convert::Infallible>>>::from_residual>
 50e3bcc:	e9 46 01 00 00       	jmp    50e3d17 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1777>
 50e3bd1:	48 8d bc 24 88 12 00 	lea    0x1288(%rsp),%rdi
 50e3bd8:	00 
 50e3bd9:	48 89 7c 24 30       	mov    %rdi,0x30(%rsp)
 50e3bde:	48 8d b4 24 d0 10 00 	lea    0x10d0(%rsp),%rsi
 50e3be5:	00 
 50e3be6:	ba d8 00 00 00       	mov    $0xd8,%edx
 50e3beb:	48 89 54 24 38       	mov    %rdx,0x38(%rsp)
 50e3bf0:	48 8b 05 09 f6 82 03 	mov    0x382f609(%rip),%rax        # 8913200 <memcpy@GLIBC_2.14>
 50e3bf7:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
 50e3bfc:	ff d0                	call   *%rax
 50e3bfe:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
 50e3c03:	48 8b 54 24 38       	mov    0x38(%rsp),%rdx
 50e3c08:	48 8b 44 24 40       	mov    0x40(%rsp),%rax
 50e3c0d:	48 8d bc 24 f8 0f 00 	lea    0xff8(%rsp),%rdi
 50e3c14:	00 
 50e3c15:	48 89 7c 24 48       	mov    %rdi,0x48(%rsp)
 50e3c1a:	ff d0                	call   *%rax
 50e3c1c:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
 50e3c21:	e8 ba d8 9b fe       	call   3aa14e0 <<tcl_compiler::signature_scan::original_name::SignatureSourceNameKey>::bytes>
 50e3c26:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
 50e3c2b:	48 89 44 24 58       	mov    %rax,0x58(%rsp)
 50e3c30:	eb 28                	jmp    50e3c5a <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x16ba>
 50e3c32:	48 8d bc 24 f8 0f 00 	lea    0xff8(%rsp),%rdi
 50e3c39:	00 
 50e3c3a:	e8 01 33 eb ff       	call   4f96f40 <core::ptr::drop_glue::<tcl_compiler::signature_scan::original_name::SignatureSourceNameKey>>
 50e3c3f:	e9 4d ef ff ff       	jmp    50e2b91 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x5f1>
 50e3c44:	48 89 c1             	mov    %rax,%rcx
 50e3c47:	89 d0                	mov    %edx,%eax
 50e3c49:	48 89 8c 24 60 13 00 	mov    %rcx,0x1360(%rsp)
 50e3c50:	00 
 50e3c51:	89 84 24 68 13 00 00 	mov    %eax,0x1368(%rsp)
 50e3c58:	eb d8                	jmp    50e3c32 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1692>
 50e3c5a:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
 50e3c5f:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
 50e3c64:	48 89 4c 24 10       	mov    %rcx,0x10(%rsp)
 50e3c69:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
 50e3c6e:	48 8b 84 24 70 03 00 	mov    0x370(%rsp),%rax
 50e3c75:	00 
 50e3c76:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
 50e3c7b:	48 8b 84 24 78 03 00 	mov    0x378(%rsp),%rax
 50e3c82:	00 
 50e3c83:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
 50e3c88:	48 8b 4c 24 28       	mov    0x28(%rsp),%rcx
 50e3c8d:	48 8b 54 24 20       	mov    0x20(%rsp),%rdx
 50e3c92:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
 50e3c97:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
 50e3c9c:	e8 df 9e b2 fd       	call   2c0db80 <<[u8]>::eq_ignore_ascii_case>
 50e3ca1:	88 44 24 0f          	mov    %al,0xf(%rsp)
 50e3ca5:	eb 00                	jmp    50e3ca7 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1707>
 50e3ca7:	48 8b 94 24 88 02 00 	mov    0x288(%rsp),%rdx
 50e3cae:	00 
 50e3caf:	48 8b bc 24 60 03 00 	mov    0x360(%rsp),%rdi
 50e3cb6:	00 
 50e3cb7:	8a 44 24 0f          	mov    0xf(%rsp),%al
 50e3cbb:	0f b6 f0             	movzbl %al,%esi
 50e3cbe:	83 e6 01             	and    $0x1,%esi
 50e3cc1:	e8 ba b0 e0 ff       	call   4eeed80 <<bool>::then::<tcl_lexer::native_word::NativeWord, <tcl_compiler::lowering::Lowerer>::original_when_event_word::{closure#6}>>
 50e3cc6:	eb 00                	jmp    50e3cc8 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1728>
 50e3cc8:	48 8d bc 24 f8 0f 00 	lea    0xff8(%rsp),%rdi
 50e3ccf:	00 
 50e3cd0:	e8 6b 32 eb ff       	call   4f96f40 <core::ptr::drop_glue::<tcl_compiler::signature_scan::original_name::SignatureSourceNameKey>>
 50e3cd5:	eb 00                	jmp    50e3cd7 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1737>
 50e3cd7:	48 8b 05 d2 05 83 03 	mov    0x38305d2(%rip),%rax        # 89142b0 <_DYNAMIC+0x12f0>
 50e3cde:	48 8d bc 24 50 0b 00 	lea    0xb50(%rsp),%rdi
 50e3ce5:	00 
 50e3ce6:	ff d0                	call   *%rax
 50e3ce8:	eb 00                	jmp    50e3cea <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x174a>
 50e3cea:	48 8d bc 24 58 04 00 	lea    0x458(%rsp),%rdi
 50e3cf1:	00 
 50e3cf2:	e8 29 8b ea ff       	call   4f8c820 <core::ptr::drop_glue::<tcl_compiler::ir::CommandTokens>>
 50e3cf7:	eb 00                	jmp    50e3cf9 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1759>
 50e3cf9:	48 8d bc 24 a0 03 00 	lea    0x3a0(%rsp),%rdi
 50e3d00:	00 
 50e3d01:	ff 15 99 f7 82 03    	call   *0x382f799(%rip)        # 89134a0 <_DYNAMIC+0x4e0>
 50e3d07:	48 8b 84 24 68 03 00 	mov    0x368(%rsp),%rax
 50e3d0e:	00 
 50e3d0f:	48 81 c4 08 14 00 00 	add    $0x1408,%rsp
 50e3d16:	c3                   	ret
 50e3d17:	e9 eb fc ff ff       	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3d1c:	e9 e6 fc ff ff       	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3d21:	e9 e1 fc ff ff       	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3d26:	e9 dc fc ff ff       	jmp    50e3a07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1467>
 50e3d2b:	48 8d bc 24 58 04 00 	lea    0x458(%rsp),%rdi
 50e3d32:	00 
 50e3d33:	e8 e8 8a ea ff       	call   4f8c820 <core::ptr::drop_glue::<tcl_compiler::ir::CommandTokens>>
 50e3d38:	eb 02                	jmp    50e3d3c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x179c>
 50e3d3a:	eb ef                	jmp    50e3d2b <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x178b>
 50e3d3c:	48 8d bc 24 a0 03 00 	lea    0x3a0(%rsp),%rdi
 50e3d43:	00 
 50e3d44:	ff 15 56 f7 82 03    	call   *0x382f756(%rip)        # 89134a0 <_DYNAMIC+0x4e0>
 50e3d4a:	eb bb                	jmp    50e3d07 <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x1767>
 50e3d4c:	eb ee                	jmp    50e3d3c <<tcl_compiler::lowering::Lowerer>::original_when_event_word+0x179c>
 50e3d4e:	48 8b bc 24 60 13 00 	mov    0x1360(%rsp),%rdi
 50e3d55:	00 
 50e3d56:	e8 85 05 f2 02       	call   80042e0 <_Unwind_Resume@plt>
