use super::SortOrder;
use rayon;
use std::cmp::Ordering;

const PARALLEL_THRESHOLD: usize = 4096;

pub fn sort<T: Ord + Send>(array: &mut [T], order: &SortOrder) -> Result<(), String> {
    // do_sortを呼ぶ代わりに、sort_by を呼ぶようにする
    // is_power_of_twoはsort_byが呼ぶので、ここからは削除した
    match *order {
        // 昇順ならa.cmp(b)、降順ならb.cmp(a)を行う
        SortOrder::Ascending => sort_by(array, &|a, b| a.cmp(b)),
        SortOrder::Descending => sort_by(array, &|a, b| b.cmp(a)),
    }
}

pub fn sort_by<T, F>(array: &mut [T], comparator: &F) -> Result<(), String>
where
    T: Send,
    F: Sync + Fn(&T, &T) -> Ordering,
{
    if array.len().is_power_of_two() {
        do_sort(array, true, comparator);
        Ok(())
    } else {
        Err(format!(
            "The length of x is not a power of two. (x.len(): {})",
            array.len()
        ))
    }
}

fn do_sort<T, F>(array: &mut [T], is_asc: bool, comparator: &F)
where
    T: Send,
    F: Sync + Fn(&T, &T) -> Ordering,
{
    if array.len() > 1 {
        let mid_point = array.len() / 2;
        // let first = &mut x[0..1];
        // let second = &mut x[2..3];
        let (first, second) = array.split_at_mut(mid_point);
        if mid_point > PARALLEL_THRESHOLD {
            // しきい値以上なら並列にソートする（並列処理）
            rayon::join(
                || do_sort(first, true, comparator),
                || do_sort(second, false, comparator),
            );
        } else {
            do_sort(first, true, comparator);
            do_sort(second, false, comparator);
        }
        sub_sort(array, is_asc, comparator);
    }
}

fn sub_sort<T, F>(array: &mut [T], is_asc: bool, comparator: &F)
where
    T: Send,
    F: Sync + Fn(&T, &T) -> Ordering,
{
    if array.len() > 1 {
        compare_and_swap(array, is_asc, comparator);
        let mid_point = array.len() / 2;
        let (first, second) = array.split_at_mut(mid_point);
        if mid_point >= PARALLEL_THRESHOLD {
            rayon::join(
                || sub_sort(first, is_asc, comparator),
                || sub_sort(second, is_asc, comparator),
            );
        } else {
            sub_sort(first, is_asc, comparator);
            sub_sort(second, is_asc, comparator);
        }
    }
}

fn compare_and_swap<T, F>(array: &mut [T], is_asc: bool, comparator: &F)
where
    F: Fn(&T, &T) -> Ordering,
{
    // 比較に先立ちforward（bool値）をOrdering値に変換しておく
    let swap_condition = if is_asc {
        Ordering::Greater
    } else {
        Ordering::Less
    };
    let mid_point = array.len() / 2;
    for i in 0..mid_point {
        // comparatorクロージャで2要素を比較し、返されたOrderingのバリアントが
        // swap_conditionと等しいなら要素を交換する
        if comparator(&array[i], &array[mid_point + i]) == swap_condition {
            array.swap(i, mid_point + i);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::{sort, sort_by};
    use crate::utils::{is_sorted_ascending, is_sorted_descending, new_u32_vec};
    use crate::SortOrder::*;

    // 構造体Studentを定義する
    // 構造体は関連する値を1つにまとめたデータ構造。複数のデータフィールドを持つ

    // deriveアトリビュートを使い、DebugトレイトとPartialEqトレイトの実装を自動導出する
    #[derive(Debug, PartialEq)]
    struct Student {
        first_name: String, // first_name（名前）フィールド。String型
        last_name: String,  // last_name（苗字）フィールド。String型
        age: u8,            // age（年齢）フィールド。u8型（8ビット符号なし整数）
    }

    // implブロックを使うと対象の型に関連関数やメソッドを実装できる
    impl Student {
        // 関連関数newを定義する
        fn new(first_name: &str, last_name: &str, age: u8) -> Self {
            // 構造体Studentを初期化して返す。Selfはimpl対象の型（Student）の別名
            Self {
                // to_stringメソッドで&str型の引数からString型の値を作る。詳しくは5章で説明
                first_name: first_name.to_string(), // first_nameフィールドに値を設定
                last_name: last_name.to_string(),   // last_nameフィールドに値を設定
                age,                                // ageフィールドにage変数の値を設定
                                                    // フィールドと変数が同じ名前のときは、このように省略形で書ける
            }
        }
    }

    #[test]
    fn sort_to_fail() {
        let mut x = vec![10, 30, 11]; // x.len() が2のべき乗になっていない。
        assert!(sort(&mut x, &Ascending).is_err());
    }

    #[test]
    fn sort_u32_ascending() {
        let mut x: Vec<u32> = vec![10, 30, 11, 20, 4, 330, 21, 110];
        assert_eq!(sort(&mut x, &Ascending), Ok(()));
        assert_eq!(x, vec![4, 10, 11, 20, 21, 30, 110, 330]);
    }

    #[test]
    fn sort_u32_descending() {
        let mut x: Vec<u32> = vec![10, 30, 11, 20, 4, 330, 21, 110];
        assert_eq!(sort(&mut x, &Descending), Ok(()));
        assert_eq!(x, vec![330, 110, 30, 21, 20, 11, 10, 4]);
    }

    #[test]
    fn sort_u32_large() {
        {
            // 乱数で65,536要素のデータ列を作る（65,536は2の16乗）
            let mut x = new_u32_vec(65536*2*2*2*2*2*2*2);
            let now = Instant::now();
            // 昇順にソートする
            assert_eq!(sort(&mut x, &Ascending), Ok(()));
            println!("sorted: {:?}", now.elapsed());
            // ソート結果が正しいことを検証する
            assert!(is_sorted_ascending(&x));
        }
        // {
        //     let mut x = new_u32_vec(65536);
        //     assert_eq!(sort(&mut x, &Descending), Ok(()));
        //     assert!(is_sorted_descending(&x));
        // }
    }

    #[test]
    fn sort_str_ascending() {
        let mut x = vec![
            "Rust",
            "is",
            "fast",
            "and",
            "memory-efficient",
            "with",
            "no",
            "GC",
        ];
        assert_eq!(sort(&mut x, &Ascending), Ok(()));
        assert_eq!(
            x,
            vec![
                "GC",
                "Rust",
                "and",
                "fast",
                "is",
                "memory-efficient",
                "no",
                "with"
            ]
        );
    }

    #[test]
    fn sort_str_descending() {
        let mut x = vec![
            "Rust",
            "is",
            "fast",
            "and",
            "memory-efficient",
            "with",
            "no",
            "GC",
        ];
        assert_eq!(sort(&mut x, &Descending), Ok(()));
        assert_eq!(
            x,
            vec![
                "with",
                "no",
                "memory-efficient",
                "is",
                "fast",
                "and",
                "Rust",
                "GC"
            ]
        );
    }

    #[test]
    // 年齢で昇順にソートする
    fn sort_students_by_age_ascending() {
        // 4人分のテストデータを作成
        let taro = Student::new("Taro", "Yamada", 16);
        let hanako = Student::new("Hanako", "Yamada", 14);
        let kyoko = Student::new("Kyoko", "Ito", 15);
        let ryosuke = Student::new("Ryosuke", "Hayashi", 17);

        // ソート対象のベクタを作成する
        let mut x = vec![&taro, &hanako, &kyoko, &ryosuke];

        // ソート後の期待値を作成する
        let expected = vec![&hanako, &kyoko, &taro, &ryosuke];

        assert_eq!(
            // sort_by関数でソートする。第2引数はソート順を決めるクロージャ
            // 引数に2つのStudent構造体をとり、ageフィールドの値をcmpメソッドで
            // 比較することで大小を決定する
            sort_by(&mut x, &|a, b| a.age.cmp(&b.age)),
            Ok(())
        );

        // 結果を検証する
        assert_eq!(x, expected);
    }

    #[test]
    fn sort_students_by_name_ascending() {
        let taro = Student::new("Taro", "Yamada", 16);
        let hanako = Student::new("Hanako", "Yamada", 14);
        let kyoko = Student::new("Kyoko", "Ito", 15);
        let ryosuke = Student::new("Ryosuke", "Hayashi", 17);

        let mut x = vec![&taro, &hanako, &kyoko, &ryosuke];
        let expected = vec![&ryosuke, &kyoko, &hanako, &taro];

        assert_eq!(
            sort_by(
                &mut x,
                // まずlast_nameを比較する
                &|a, b| a
                    .last_name
                    .cmp(&b.last_name)
                    // もしlast_nameが等しくない（LessまたはGreater）ならそれを返す
                    // last_nameが等しい（Equal）ならfirst_nameを比較する
                    .then_with(|| a.first_name.cmp(&b.first_name))
            ),
            Ok(())
        );
        assert_eq!(x, expected);
    }
}
