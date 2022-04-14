using System.Reflection;
using System;
using System.Linq;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using OneOf;

using Il2CppRef = Il2CppSystem.Reflection;

using Il2Type = UnhollowerRuntimeLib.Il2CppType;

namespace vrc_udon_shit {
    class NamespaceTree : Dictionary<string, StrongBox<(NamespaceTree, Il2CppSystem.Type)>> {
        public bool AddType(Il2CppSystem.Type ty) {
            var cur_val = new StrongBox<(NamespaceTree, Il2CppSystem.Type)>((this, null));
            try {
                if (ty.Namespace is null) {
                    return false; // most likely a <PrivateImplementationDetails> ..., not relevant.
                }

                foreach (var segment in ty.FullName.Split('.', '+')) {
                    var segmentn = segment;
                    if (segment.EndsWith("`1")) {
                        segmentn = segment.Substring(0, segment.Length - 2);
                    }
                    if (!cur_val.Value.Item1.TryGetValue(segmentn, out var value)) {
                        value = new StrongBox<(NamespaceTree, Il2CppSystem.Type)>((new NamespaceTree(), null));
                        cur_val.Value.Item1.Add(segmentn, value);
                    }
                    cur_val = value;
                }
            } catch (Exception ex) {
                UdonShit.logger.Warning($"Failed to add type {ty.FullName}: {ex}");
                return false;
            }

            cur_val.Value.Item2 = ty;
            return true;
        }

        private Il2CppSystem.Type GetFromConcatImpl(NamespaceTree root, string concat_name) {
            foreach (var segment_name in this.Keys) {
                var value = this[segment_name];
                if (concat_name == segment_name) {
                    if (value.Value.Item2 != null) {
                        return value.Value.Item2;
                    }
                } else if (concat_name.StartsWith(segment_name)) {
                    var subname = concat_name.Substring(segment_name.Length);
                    var result = value.Value.Item1.GetFromConcatImpl(root, subname);
                    if (result != null) {
                        return result;
                    } else if (!(value.Value.Item2 is null) && subname == "Array") {
                        return value.Value.Item2.MakeArrayType();
                    } else if (value.Value.Item2?.IsGenericType ?? false) {
                        // attempt to handle generic types with a single argument.
                        UdonShit.logger.Msg($"attempting generic {value.Value.Item2.FullName} -> {subname}");
                        var generic_param = root.GetFromConcat(subname);
                        if (!(generic_param is null)) {
                            return value.Value.Item2.MakeGenericType(new Il2CppSystem.Type[] { generic_param });
                        }
                    }
                }
            }

            return null;
        }

        public Il2CppSystem.Type GetFromConcat(string concat_name) {
            return GetFromConcatImpl(this, concat_name);
        }
    }

    public class UdonModules {
        // UnSystemify is from AssemblyUnhollower
        // private static List<string> UnSystemifyNamespaces = new List<string>() {"System", "mscorlib", "Microsoft", "Mono", "I18N"};
        // private static string UnSystemify(string str) {
        //     foreach (var prefix in UnSystemifyNamespaces)
        //         if (str.StartsWith(prefix))
        //             return "Il2Cpp" + str;
        //     return str;
        // }
        private static string ToConcat(Il2CppSystem.Type ty, ref int priority) {
            if (ty.Namespace == "Unity.Collections" && ty.Name == "NativeArray`1") {
                priority += 1; // lower priority is preferred.
                return ToConcat(ty.GenericTypeArguments[0], ref priority) + "Array";
            }

            if (ty.IsArray) {
                return ToConcat(ty.GetElementType(), ref priority) + "Array";
            }

            if (ty.IsByRef) {
                return ToConcat(ty.GetElementType(), ref priority) + "Ref";
            }

            if (ty.Namespace is null) {
                // generic!
                priority += 1;
                var genericName = "<>";
                if (ty.IsArray) {
                    genericName += "Array";
                }
                if (ty.IsByRef) {
                    genericName += "Ref";
                }
                return genericName;
            }

            var ns = ty.Namespace.Replace(".", "");
            if (!(ty.DeclaringType is null)) {
                ns = ToConcat(ty.DeclaringType, ref priority);
            }
            
            var name = ty.Name;

            if (ty.GenericTypeArguments.Length != 0) {
                name = name.Substring(0, name.Length - 2);
            }

            int inner_priority = 0;

            var res = ns + name + String.Concat(ty.GenericTypeArguments.Select(a => ToConcat(a, ref inner_priority)));

            priority += inner_priority;

            switch (res) {
                case "SystemCollectionsGenericList<>":
                    return "List<>";
                case "SystemCollectionsGenericIEnumerable<>":
                    return "IEnumerable<>";
                default:
                    return res;
            }
        }

        private static Regex ConcatToRegex(string c) {
            // disable checking for generics
            return new Regex('^' + /*c.Replace("<>", "[a-zA-Z]*")*/ c + '$');
        }

        private static (Il2CppRef.MethodInfo, Regex, int) MethodToConcat(Il2CppRef.MethodInfo method) {
            var res = "";
            int priority = 0;
            if (method.GetParametersCount() != 0) {
                res = String.Join("_", method.GetParameters().Select(p => ToConcat(p.ParameterType, ref priority))) + "__";
            }
            return (method, ConcatToRegex(res + ToConcat(method.ReturnType, ref priority)), priority);
        }

        // bunch of places where original is generic type and udon is specific type. (FUCK)
        //   -> can we detect this (i.e. regex pattern matching)? or should we just hardcode them?
        // * regex matching was somewhat successful, but i'm not sure that it can be applied to a good result. Time will tell.

        public static T Identity<T>(T item) => item;

        public static void TestFn() {
            // include our special cases from the beginning:
            var generated = Hardcoded.SpecialCases.ToDictionary(i => i.Key, i => i.Value);
            var VRCUdonWrapperTypes = Assembly.GetAssembly(typeof(VRC.Udon.Wrapper.Modules.ExternSystemBoolean)).GetTypes();
            var VRCUdonVRCWrapperModules = Assembly.GetAssembly(typeof(VRC.Udon.Wrapper.Modules.ExternVRCSDKBaseUtilities)).GetTypes();

            var vrc_wrappers = VRCUdonWrapperTypes.Concat(VRCUdonVRCWrapperModules)
                .Where(t => {
                    if (t.Namespace != "VRC.Udon.Wrapper.Modules") {
                        return false;
                    }

                    if (!t.Name.StartsWith("Extern")) {
                        UdonShit.logger.Warning($"Unknown wrapper module prefix on module {t.Name}!");
                        return false;
                    }

                    return true;
                }).ToList();

            var namespace_tree = new NamespaceTree();

            var types = Il2CppSystem.AppDomain.CurrentDomain.GetAssemblies()
                    .SelectMany(a => a.GetTypes());

            foreach (var ty in types) {
                namespace_tree.AddType(ty);
            }

            var concat_map = new Dictionary<string, Il2CppSystem.Type>();
            
            foreach (var type in vrc_wrappers) {

                var concat_name = type.Name.Substring("Extern".Length);

                var il2cppType = namespace_tree.GetFromConcat(concat_name);

                if (il2cppType is null) {
                    UdonShit.logger.Warning($"Unable to find il2cpp type for {type.Name}");
                    continue;
                }

                concat_map.Add(concat_name, il2cppType);
            }

            //var file = new System.IO.StreamWriter("test.txt");

            foreach (var wrapper in vrc_wrappers) {
                var il2cppType = concat_map[wrapper.Name.Substring("Extern".Length)];
                if (il2cppType is null) {
                    UdonShit.logger.Error("WTF");
                    continue;
                }
                foreach (var method in wrapper.GetMethods(BindingFlags.Public | BindingFlags.Instance)) {
                    if (!method.Name.StartsWith("__")) {
                        continue;
                    }

                    var udon_key = wrapper.Name.Substring("Extern".Length) + '.' + method.Name;
                    if (Hardcoded.SpecialCases.ContainsKey(udon_key)) {
                        UdonShit.logger.Msg($"Found special case {wrapper.Name.Substring("Extern".Length) + '.' + method.Name}");
                        continue;
                    }

                    var tmp = method.Name.Split(new string[] { "__" }, 2, StringSplitOptions.RemoveEmptyEntries);

                    var name = tmp[0];
                    var rest = tmp[1];

                    int tmp_priority = 0;

                    if (name == "ctor") {
                        var ctors = il2cppType.GetConstructors()
                            .Select(c => {
                                var res = "";
                                int priority = 0;
                                if (c.GetParametersCount() != 0) {
                                    res = String.Join("_", c.GetParameters().Select(p => ToConcat(p.ParameterType, ref priority))) + "__";
                                }
                                return (c, ConcatToRegex(res + ToConcat(il2cppType, ref priority)), priority);
                            });

                        var possible_ctors = ctors.Where(c => c.Item2.IsMatch(rest)).ToList();

                        if (possible_ctors.Count != 1) {
                            UdonShit.logger.Error($"Found {possible_ctors.Count} possible ctors for {il2cppType.FullName}'s {method.Name}. Candidites:");
                            foreach (var ctor in ctors) {
                                UdonShit.logger.Error($"    __ctor__{ctor.Item2}");
                            }
                        } else {
                            var c = possible_ctors[0].Item1;

                            generated.Add(udon_key, c);
                            continue;
                        }
                    } else if (name.StartsWith("get_") || name.StartsWith("set_")) {
                        var field_name = name.Substring(4);
                        var field_info = il2cppType.GetField(field_name);
                        if (field_info is null) {
                            // time to try properties
                            var properties = il2cppType.GetProperties()
                                .Where(p => p.Name == field_name)
                                .Select(p => {
                                    Il2CppRef.MethodInfo method;
                                    if (name.StartsWith("get_")) {
                                        // getter
                                        method = p.GetGetMethod();
                                    } else {
                                        // setter
                                        method = p.GetSetMethod();
                                    }
                                    return MethodToConcat(method);
                                });
                            
                            // TODO: this is a duplicate of some of the logic used for methods.
                            var possible_properties = properties.Where(p => p.Item2.IsMatch(rest)).ToList();

                            if (possible_properties.Count > 1) {
                                var min_priority = possible_properties.Select(p => p.Item3).Min();
                                possible_properties = possible_properties.Where(p => p.Item3 == min_priority).ToList();
                            }

                            if (possible_properties.Count > 1) {
                                // prioritize method on current type instead of inherited type, where applicable.
                                possible_properties = possible_properties.Where(m => m.Item1.DeclaringType == il2cppType).ToList();
                            }

                            if (possible_properties.Count == 0 && properties.ToList().Count == 0) {
                                UdonShit.logger.Warning($"{name} inlined or method on {il2cppType.FullName} ({method.Name})");
                            } else if (possible_properties.Count != 1) {
                                UdonShit.logger.Error($"Found {possible_properties.Count} possible properties for {il2cppType.FullName}'s {method.Name}. Candidates:");
                                foreach (var m in properties) {
                                    UdonShit.logger.Error($"    {m.Item2}");
                                }
                            } else {
                                var m = possible_properties[0].Item1;
                                generated.Add(udon_key, m);
                                continue;
                            }
                        } else if (rest == ToConcat(field_info.FieldType, ref tmp_priority)) {
                            // we have a field, and it matches
                            generated.Add(udon_key, new CodegenInfo.FieldGetSet() { field = field_info, isSet = name.StartsWith("set_") });
                            continue;
                        } else {
                            UdonShit.logger.Error($"Mismatched type! Got {field_info.FieldType.FullName}. ({method.Name})");
                        }
                    } else if ((il2cppType.IsPrimitive || il2cppType.FullName == "System.Object" || (il2cppType.FullName == "System.String" && name == "op_Addition")) && name.StartsWith("op_")) {
                        // these do not overload the operators, they're just already defined.
                        // TODO: check to make sure the operators are what we think they should be:
                        generated.Add(udon_key, new CodegenInfo.PrimitiveOp() { type = il2cppType, op = name });
                        continue;
                    } else {
                        // FlattenHierarchy, to include public static operators from parent types.
                        var methods = il2cppType.GetMethods(Il2CppRef.BindingFlags.FlattenHierarchy | Il2CppRef.BindingFlags.Public | Il2CppRef.BindingFlags.Instance | Il2CppRef.BindingFlags.Static)
                            .Where(m => m.Name == tmp[0])
                            .Select(MethodToConcat);

                        var possible_methods = methods.Where(m => m.Item2.IsMatch(rest)).ToList();

                        if (possible_methods.Count > 1) {
                            var min_priority = possible_methods.Select(p => p.Item3).Min();
                            possible_methods = possible_methods.Where(p => p.Item3 == min_priority).ToList();
                        }

                        if (possible_methods.Count > 1) {
                            // prioritize method on current type instead of inherited type, where applicable.
                            possible_methods = possible_methods.Where(m => m.Item1.DeclaringType == il2cppType).ToList();
                        }
                        
                        if (possible_methods.Count != 1) {
                            UdonShit.logger.Error($"Found {possible_methods.Count} possible methods for {il2cppType.FullName}'s {method.Name}. Candidates:");
                            foreach (var m in methods) {
                                UdonShit.logger.Error($"    {m.Item2}");
                            }
                        } else {
                            var m = possible_methods[0].Item1;
                            generated.Add(udon_key, m);
                            continue;
                        }

                    }

                    // handle stuff that isn't otherwise handled?
                }
            }

            //file.Flush();
        }
    }
}